#!/usr/bin/env bash
# ==============================================================================
# Cloudflare Serverless Deployment Build Pipeline Script (Pages & Workers)
# Universal Rust Algebra Engine (URAE)
#
# Usage:
#   ./deploy.sh                  # Pure Static Site (Default, no Edge Worker API)
#   ./deploy.sh --static-only    # Explicit Pure Static Site
#   ./deploy.sh --with-api       # Full-Stack Mode (Static Site + Cloudflare Pages Worker API)
#   ./deploy.sh --worker-deploy  # Standalone Cloudflare Worker Deploy (Wrangler)
# ==============================================================================
set -euo pipefail

ENABLE_EDGE_API="${ENABLE_EDGE_API:-false}"
CLOUDFLARE_WORKER_DEPLOY="${CLOUDFLARE_WORKER_DEPLOY:-false}"

for arg in "$@"; do
    case "$arg" in
        --static-only)
            ENABLE_EDGE_API="false"
            CLOUDFLARE_WORKER_DEPLOY="false"
            ;;
        --with-api)
            ENABLE_EDGE_API="true"
            ;;
        --worker-deploy)
            CLOUDFLARE_WORKER_DEPLOY="true"
            ;;
    esac
done

if [ "$ENABLE_EDGE_API" = "true" ]; then
    echo "=== Initializing Cloudflare Build: Full-Stack Mode (Static Site + Edge Worker API) ==="
elif [ "$CLOUDFLARE_WORKER_DEPLOY" = "true" ]; then
    echo "=== Initializing Cloudflare Build: Standalone Wrangler Worker Mode ==="
else
    echo "=== Initializing Cloudflare Build: Pure Static Site Mode (No Serverless API) ==="
fi

# 1. Persistent Environment & PATH Setup
export NODE_ENV="production"
export NODE_VERSION="${NODE_VERSION:-24}"

if [ -d "/opt/buildhome" ]; then
    export CARGO_HOME="/opt/buildhome/.cargo"
    export RUSTUP_HOME="/opt/buildhome/.rustup"
else
    export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
    export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
fi

export PATH="$CARGO_HOME/bin:$HOME/.cargo/bin:/opt/buildhome/.cargo/bin:$PATH"
mkdir -p "$CARGO_HOME/bin"

SPODEIAN_GIT_TOKEN="${SPODEIAN_CORE_TOKEN:-${PRIVATE_READ_ACCESS:-${GH_PAT:-}}}"
if [ -n "$SPODEIAN_GIT_TOKEN" ]; then
    export CARGO_NET_GIT_FETCH_WITH_CLI=true
    export GIT_CONFIG_COUNT=1
    export GIT_CONFIG_KEY_0="url.https://x-access-token:${SPODEIAN_GIT_TOKEN}@github.com/.insteadOf"
    export GIT_CONFIG_VALUE_0="https://github.com/"
fi

if [ -f "$CARGO_HOME/env" ]; then
    . "$CARGO_HOME/env"
elif [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

# 2. Rust Toolchain & Target Verification
RUST_TOOLCHAIN="1.95.0"

if ! command -v rustup &> /dev/null && [ ! -f "$CARGO_HOME/bin/rustup" ]; then
    echo "Rust compiler not detected. Installing Rust $RUST_TOOLCHAIN minimal toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$RUST_TOOLCHAIN" --profile minimal --target wasm32-unknown-unknown
    if [ -f "$CARGO_HOME/env" ]; then
        . "$CARGO_HOME/env"
    fi
else
    echo "Rust toolchain detected: $(rustc --version || echo 'Active')"
    if command -v rustup &> /dev/null; then
        rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --target wasm32-unknown-unknown 2>/dev/null || true
        rustup default "$RUST_TOOLCHAIN" 2>/dev/null || true
        rustup target add wasm32-unknown-unknown 2>/dev/null || true
    elif [ -f "$CARGO_HOME/bin/rustup" ]; then
        "$CARGO_HOME/bin/rustup" toolchain install "$RUST_TOOLCHAIN" --profile minimal --target wasm32-unknown-unknown 2>/dev/null || true
        "$CARGO_HOME/bin/rustup" default "$RUST_TOOLCHAIN" 2>/dev/null || true
        "$CARGO_HOME/bin/rustup" target add wasm32-unknown-unknown 2>/dev/null || true
    fi
fi

# 3. wasm-bindgen CLI Installation (Matches Cargo.lock exact version for zero linker errors)
WASM_BINDGEN_VERSION=$(grep -A 1 'name = "wasm-bindgen"' Cargo.lock 2>/dev/null | grep 'version =' | head -n 1 | cut -d '"' -f 2 || echo "0.2.115")
if [ -z "$WASM_BINDGEN_VERSION" ]; then
    WASM_BINDGEN_VERSION="0.2.115"
fi

WASM_BINDGEN_BIN="$CARGO_HOME/bin/wasm-bindgen"
if [ -x "$WASM_BINDGEN_BIN" ] && "$WASM_BINDGEN_BIN" --version 2>&1 | grep -q "$WASM_BINDGEN_VERSION"; then
    echo "Cached wasm-bindgen detected: $("$WASM_BINDGEN_BIN" --version)"
elif command -v wasm-bindgen &> /dev/null && wasm-bindgen --version 2>&1 | grep -q "$WASM_BINDGEN_VERSION"; then
    echo "System wasm-bindgen detected: $(wasm-bindgen --version)"
    WASM_BINDGEN_BIN="wasm-bindgen"
else
    echo "Downloading and caching wasm-bindgen v${WASM_BINDGEN_VERSION} CLI binary..."
    temp_wb="/tmp/wasm-bindgen-${WASM_BINDGEN_VERSION}.tar.gz"
    wget -qO "$temp_wb" "https://github.com/rustwasm/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl.tar.gz" || \
    wget -qO "$temp_wb" "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl.tar.gz" || true
    if [ -f "$temp_wb" ] && [ -s "$temp_wb" ]; then
        tar -xzf "$temp_wb" -C /tmp
        find /tmp -name "wasm-bindgen" -type f -exec mv {} "$CARGO_HOME/bin/wasm-bindgen" \;
        chmod +x "$CARGO_HOME/bin/wasm-bindgen"
        rm -rf "$temp_wb" /tmp/wasm-bindgen*
    fi
    if [ ! -x "$CARGO_HOME/bin/wasm-bindgen" ]; then
        echo "Precompiled binary unavailable. Installing wasm-bindgen-cli via cargo..."
        cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --root "$CARGO_HOME"
    fi
    WASM_BINDGEN_BIN="$CARGO_HOME/bin/wasm-bindgen"
    echo "wasm-bindgen installed: $("$WASM_BINDGEN_BIN" --version || echo 'Ready')"
fi

# 4. Binaryen (wasm-opt) (Check Cache & Validate Execution)
WASM_OPT_BIN="$CARGO_HOME/bin/wasm-opt"

if [ -x "$WASM_OPT_BIN" ] && "$WASM_OPT_BIN" --version &> /dev/null; then
    echo "Cached wasm-opt detected: $("$WASM_OPT_BIN" --version)"
elif command -v wasm-opt &> /dev/null && wasm-opt --version &> /dev/null; then
    echo "System wasm-opt detected: $(wasm-opt --version)"
    WASM_OPT_BIN="wasm-opt"
else
    echo "Downloading and caching Binaryen wasm-opt..."
    BINARYEN_VERSION="version_132"
    temp_tar="/tmp/binaryen-${BINARYEN_VERSION}.tar.gz"
    wget -qO "$temp_tar" "https://github.com/WebAssembly/binaryen/releases/download/${BINARYEN_VERSION}/binaryen-${BINARYEN_VERSION}-x86_64-linux.tar.gz" || \
    wget -qO "$temp_tar" "https://github.com/WebAssembly/binaryen/releases/latest/download/binaryen-x86_64-linux.tar.gz"
    tar -xzf "$temp_tar" -C /tmp
    find /tmp -name "wasm-opt" -type f -exec mv {} "$CARGO_HOME/bin/wasm-opt" \;
    chmod +x "$CARGO_HOME/bin/wasm-opt"
    rm -rf "$temp_tar" /tmp/binaryen*
    WASM_OPT_BIN="$CARGO_HOME/bin/wasm-opt"
    echo "wasm-opt installed: $("$WASM_OPT_BIN" --version || echo 'Ready')"
fi

# 5. Clean & Build WebAssembly Application
echo "Purging previous build distribution caches..."
rm -rf crates/urae-wasm/public/pkg crates/urae-wasm/pkg dist

export RUSTFLAGS="-C target-feature=+simd128,+bulk-memory,+mutable-globals,+nontrapping-fptoint,+sign-ext -C link-arg=-zstack-size=2097152 ${RUSTFLAGS:-}"

echo "Compiling WebAssembly release with Cargo (toolchain: $RUST_TOOLCHAIN, bulk-memory enabled)..."
if command -v rustup &> /dev/null; then
    rustup run "$RUST_TOOLCHAIN" cargo build --target wasm32-unknown-unknown -p urae-wasm --release
elif [ -f "$CARGO_HOME/bin/rustup" ]; then
    "$CARGO_HOME/bin/rustup" run "$RUST_TOOLCHAIN" cargo build --target wasm32-unknown-unknown -p urae-wasm --release
else
    cargo build --target wasm32-unknown-unknown -p urae-wasm --release
fi

echo "Generating WebAssembly bindings via wasm-bindgen..."
mkdir -p crates/urae-wasm/public/pkg
if [ -x "$WASM_BINDGEN_BIN" ] || command -v wasm-bindgen &> /dev/null; then
    "${WASM_BINDGEN_BIN:-wasm-bindgen}" target/wasm32-unknown-unknown/release/urae_wasm.wasm --target web --out-dir crates/urae-wasm/public/pkg
else
    cd crates/urae-wasm
    wasm-pack build --target web --out-dir public/pkg --release
    cd ../..
fi

DIST_DIR="crates/urae-wasm/public"

# Run wasm-opt pass on generated wasm artifact with bulk memory, reference-types, and performance optimizations
WASM_OPT_FLAGS=(
    "-Oz"
    "--enable-simd"
    "--enable-bulk-memory"
    "--enable-bulk-memory-opt"
    "--enable-mutable-globals"
    "--enable-sign-ext"
    "--enable-nontrapping-float-to-int"
    "--enable-reference-types"
    "--enable-multivalue"
)

if [ -x "$WASM_OPT_BIN" ] || command -v wasm-opt &> /dev/null; then
    for wasm_file in "$DIST_DIR"/pkg/*.wasm; do
        if [ -f "$wasm_file" ]; then
            echo "Optimizing WASM with wasm-opt (bulk-memory, fast math & performance flags): $wasm_file"
            "$WASM_OPT_BIN" "${WASM_OPT_FLAGS[@]}" "$wasm_file" -o "$wasm_file" || "$WASM_OPT_BIN" -Oz --enable-simd "$wasm_file" -o "$wasm_file" || true
        fi
    done
fi

# 6. Deployment Cache Invalidation & Version Stamping
BUILD_ID=$(git rev-parse --short HEAD 2>/dev/null || date +%s)
echo "=== Stamping Deployment Cache Invalidation with BUILD_ID: $BUILD_ID ==="
if [ -f "$DIST_DIR/sw.js" ]; then
    sed -i "s/CACHE_NAME = '.*'/CACHE_NAME = 'urae-cache-${BUILD_ID}'/g" "$DIST_DIR/sw.js" 2>/dev/null || true
    sed -i "s/?v=[a-zA-Z0-9_-]*/?v=${BUILD_ID}/g" "$DIST_DIR/sw.js" 2>/dev/null || true
fi
if [ -f "$DIST_DIR/index.html" ]; then
    sed -i "s/?v=[a-zA-Z0-9_-]*/?v=${BUILD_ID}/g" "$DIST_DIR/index.html" 2>/dev/null || true
fi

# 7. Production Asset Minification (HTML, CSS, JS)
if [ -d "$DIST_DIR" ]; then
    echo "=== Running Production Asset Minification (HTML, CSS, JS) for '$DIST_DIR' ==="

    if command -v npx &> /dev/null; then
        echo "Minifying JavaScript and CSS assets using esbuild (Node $(node -v 2>/dev/null || echo 'unknown'))..."
        for js_file in "$DIST_DIR"/*.js; do
            if [ -f "$js_file" ]; then
                echo "  Minifying JS: $js_file"
                npx --yes esbuild "$js_file" --minify --allow-overwrite --outfile="$js_file" 2>/dev/null || true
            fi
        done
        for scss_file in "$DIST_DIR"/*.scss; do
            if [ -f "$scss_file" ]; then
                css_file="${scss_file%.scss}.css"
                echo "  Compiling SCSS: $scss_file -> $css_file"
                npx --yes sass "$scss_file" "$css_file" --no-source-map 2>/dev/null || true
            fi
        done
        for css_file in "$DIST_DIR"/*.css; do
            if [ -f "$css_file" ]; then
                echo "  Minifying CSS: $css_file"
                npx --yes esbuild "$css_file" --minify --allow-overwrite --outfile="$css_file" 2>/dev/null || true
            fi
        done
        if [ -f "$DIST_DIR/index.html" ]; then
            echo "  Minifying HTML: $DIST_DIR/index.html"
            npx --yes html-minifier-terser --collapse-whitespace --remove-comments --remove-redundant-attributes --remove-script-type-attributes --remove-style-link-type-attributes --use-short-doctype --minify-css true --minify-js true -o "$DIST_DIR/index.html" "$DIST_DIR/index.html" 2>/dev/null || true
        fi
    elif command -v python3 &> /dev/null; then
        echo "Node/npx not available. Using Python minification engine fallback..."
        python3 -c '
import os, re, sys, glob

dist_dir = sys.argv[1]

def minify_css(content):
    content = re.sub(r"/\*[\s\S]*?\*/", "", content)
    content = re.sub(r"\s+", " ", content)
    content = re.sub(r"\s*([\{\}:;,])\s*", r"\1", content)
    content = content.replace(";}", "}")
    return content.strip()

def minify_js(content):
    lines = []
    for line in content.splitlines():
        stripped = line.strip()
        if stripped.startswith("//") and not stripped.startswith("///"):
            continue
        lines.append(line)
    content = "\n".join(lines)
    content = re.sub(r"/\*[\s\S]*?\*/", "", content)
    content = re.sub(r"[ \t]+", " ", content)
    content = re.sub(r"\n\s*", "\n", content)
    content = re.sub(r"\s*([=+\-*/%&|!<>?:,;{}()[\]])\s*", r"\1", content)
    return content.strip()

for fpath in glob.glob(os.path.join(dist_dir, "*.css")):
    try:
        with open(fpath, "r", encoding="utf-8") as f:
            c = f.read()
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(minify_css(c))
        print(f"  Minified CSS: {fpath}")
    except Exception as e:
        print(f"  Error minifying {fpath}: {e}")

for fpath in glob.glob(os.path.join(dist_dir, "**", "*.js"), recursive=True):
    try:
        with open(fpath, "r", encoding="utf-8") as f:
            c = f.read()
        with open(fpath, "w", encoding="utf-8") as f:
            f.write(minify_js(c))
        print(f"  Minified JS: {fpath}")
    except Exception as e:
        print(f"  Error minifying {fpath}: {e}")

html_path = os.path.join(dist_dir, "index.html")
if os.path.exists(html_path):
    try:
        with open(html_path, "r", encoding="utf-8") as f:
            html = f.read()
        html = re.sub(r"<!--(?!\[if)[\s\S]*?-->", "", html)
        html = re.sub(r"<style[^>]*>([\s\S]*?)</style>", lambda m: f"<style>{minify_css(m.group(1))}</style>", html, flags=re.IGNORECASE)
        html = re.sub(r">\s+<", "><", html)
        html = re.sub(r"[ \t]+", " ", html)
        with open(html_path, "w", encoding="utf-8") as f:
            f.write(html.strip())
        print(f"  Minified HTML: {html_path}")
    except Exception as e:
        print(f"  Error processing {html_path}: {e}")
' "$DIST_DIR"
    fi

    # 7. High-Ratio Asset Pre-Compression (Brotli Level 11 + Gzip Level 9)
    echo "=== Generating Pre-Compressed Brotli (.br) & Gzip (.gz) Assets ==="
    if command -v python3 &> /dev/null; then
        python3 -c '
import os, sys, gzip, glob

dist_dir = sys.argv[1]
extensions = ("*.wasm", "*.js", "*.css", "*.html", "*.json", "*.svg")
target_files = []
for root, _, _ in os.walk(dist_dir):
    for ext in extensions:
        target_files.extend(glob.glob(os.path.join(root, ext)))

# 1. Gzip Level 9
for fpath in target_files:
    if fpath.endswith(".gz") or fpath.endswith(".br"): continue
    gz_path = fpath + ".gz"
    try:
        with open(fpath, "rb") as f_in, gzip.open(gz_path, "wb", compresslevel=9) as f_out:
            f_out.write(f_in.read())
    except Exception as e:
        print(f"  Gzip failed for {fpath}: {e}")

# 2. Brotli Level 11 (if brotli module is available)
try:
    import brotli
    for fpath in target_files:
        if fpath.endswith(".gz") or fpath.endswith(".br"): continue
        br_path = fpath + ".br"
        with open(fpath, "rb") as f_in:
            data = f_in.read()
        compressed = brotli.compress(data, quality=11, mode=brotli.MODE_GENERIC)
        with open(br_path, "wb") as f_out:
            f_out.write(compressed)
    print("  Successfully pre-compressed assets with Brotli (q11) & Gzip (level 9)")
except ImportError:
    print("  Pre-compressed assets with Gzip (level 9). Brotli CLI check...")
' "$DIST_DIR" || true
        if command -v brotli &> /dev/null; then
            find "$DIST_DIR" -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.css" -o -name "*.html" -o -name "*.json" -o -name "*.svg" \) -exec brotli -f -k -q 11 {} + 2>/dev/null || true
        fi
    fi

    # 8. Configure Cloudflare Workers Static Assets + Edge Worker deployment
    cp -f crates/urae-wasm/public/_headers "$DIST_DIR/_headers" 2>/dev/null || true
    cp -f crates/urae-wasm/public/_redirects "$DIST_DIR/_redirects" 2>/dev/null || true
    # Ensure Worker entrypoints and legacy Pages routing manifests are never exposed as static assets
    rm -f "$DIST_DIR/_worker.js" "$DIST_DIR/_routes.json"
fi

echo "=== Build Completed Successfully! Output directory ready in: '$DIST_DIR' ==="

# 9. Deployment Context Router
if [ "${CLOUDFLARE_WORKER_DEPLOY:-false}" = "true" ]; then
    echo "Executing Cloudflare Workers Deploy..."
    if command -v wrangler &> /dev/null; then
        wrangler deploy
    else
        npx --yes wrangler@4 deploy
    fi
else
    echo "Workers Static Assets build complete. Deploy with: npx --yes wrangler@4 deploy"
fi
