import os
import re

crates_dir = r"C:\Users\Liam\Code\algebra-engine\crates"

pattern_unwrap = re.compile(r'\.unwrap\(\)')
pattern_expect = re.compile(r'\.expect\(')
pattern_panic = re.compile(r'panic!\(')
pattern_unreach = re.compile(r'unreachable!\(')

for c in sorted(os.listdir(crates_dir)):
    src_dir = os.path.join(crates_dir, c, "src")
    if not os.path.isdir(src_dir):
        continue
    for root, dirs, files in os.walk(src_dir):
        for f in files:
            if not f.endswith(".rs"):
                continue
            path = os.path.join(root, f)
            rel_path = os.path.relpath(path, r"C:\Users\Liam\Code\algebra-engine")
            with open(path, "r", encoding="utf-8", errors="ignore") as fp:
                lines = fp.readlines()
            
            in_test_module = False
            for idx, line in enumerate(lines, 1):
                stripped = line.strip()
                if stripped.startswith("//"):
                    continue
                if "mod test" in stripped or "#[cfg(test)]" in stripped:
                    in_test_module = True
                
                if in_test_module:
                    continue

                matches = []
                if pattern_unwrap.search(line):
                    matches.append("unwrap")
                if pattern_expect.search(line):
                    matches.append("expect")
                if pattern_panic.search(line):
                    matches.append("panic!")
                if pattern_unreach.search(line):
                    matches.append("unreachable!")
                
                if matches:
                    print(f"{rel_path}:{idx}: {', '.join(matches)} -> {stripped}")
