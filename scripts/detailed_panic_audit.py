import os
import re

crates_dir = r"C:\Users\Liam\Code\algebra-engine\crates"

# Common patterns that can panic in Rust:
# 1. Direct indexing: expr[idx] (where expr is ident or field, not a slice/type definition)
# 2. Division: / and %
# 3. .unwrap(), .expect(), panic!(), unreachable!(), unimplemented!(), todo!()
# 4. slice/array indexing without bounds check

# Let's inspect functions in the core computational crates:
# algebra-core, algebra-engine, algebra-advanced, cat-*, urae-cli, urae

target_crates = [
    "algebra-core", "algebra-engine", "algebra-advanced", "cat-core",
    "cat-numbers", "cat-tangent", "cat-egraph", "cat-lang", "cat-bridge",
    "cat-anytime", "auto-promoters", "urae", "urae-cli", "urae-notebook"
]

print("Scanning for indexing and potentially unchecked operations...")
