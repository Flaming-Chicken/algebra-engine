import os
import re

crates_dir = r"C:\Users\Liam\Code\algebra-engine\crates"
results = []

for c in sorted(os.listdir(crates_dir)):
    src_dir = os.path.join(crates_dir, c, "src")
    if not os.path.isdir(src_dir):
        continue
    c_unwraps = 0
    c_expects = 0
    c_panics = 0
    c_unreach = 0
    c_todo = 0
    files_count = 0
    for root, dirs, files in os.walk(src_dir):
        for f in files:
            if f.endswith(".rs"):
                files_count += 1
                path = os.path.join(root, f)
                with open(path, "r", encoding="utf-8", errors="ignore") as fp:
                    content = fp.read()
                    # Exclude cfg(test) blocks roughly or count everywhere
                    c_unwraps += len(re.findall(r'\.unwrap\(\)', content))
                    c_expects += len(re.findall(r'\.expect\(', content))
                    c_panics += len(re.findall(r'panic!\(', content))
                    c_unreach += len(re.findall(r'unreachable!\(', content))
                    c_todo += len(re.findall(r'(?:todo|unimplemented)!\(', content))
    results.append((c, files_count, c_unwraps, c_expects, c_panics, c_unreach, c_todo))

print(f"{'Crate':<20} | {'Files':<5} | {'unwrap':<7} | {'expect':<7} | {'panic':<7} | {'unreach':<7} | {'todo':<5}")
print("-" * 70)
for r in results:
    print(f"{r[0]:<20} | {r[1]:<5} | {r[2]:<7} | {r[3]:<7} | {r[4]:<7} | {r[5]:<7} | {r[6]:<5}")
