import sys
import csv

rows = int(sys.argv[1])
out = sys.argv[2]

with open(out, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["id", "value"])
    for i in range(rows):
        w.writerow([i, (i % 1000) + 0.5])

print(f"wrote {rows} rows to {out}")
