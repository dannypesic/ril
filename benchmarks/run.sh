#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

ROWS="${1:-1000000}"
RIL_BIN="../target/release/ril"
PY=".venv/bin/python"

echo "building ril (release)"
( cd .. && cargo build --release >/dev/null 2>&1 )
cp "$RIL_BIN" ./ril

echo "initializing .venv and generating input.csv ($ROWS rows)"
if [ ! -d .venv ]; then
  python3 gen_data.py 10 input.csv >/dev/null
  ./ril >/dev/null 2>&1 || true
fi
"$PY" -m pip install -q pandas >/dev/null 2>&1 || true
"$PY" gen_data.py "$ROWS" input.csv

timeit() { local label="$1"; shift; "$PY" mem_bench.py "$label" -- "$@"; }

echo
echo "ROWS=$ROWS   cores=$(sysctl -n hw.logicalcpu)   workload K=$(awk -F= '/^K =/{print $2}' workload.py | tr -d ' ')"
timeit "single core (pandas)"        "$PY" serial.py input.csv out_serial.csv
timeit "multiprocessing.Pool"        "$PY" multi.py  input.csv out_multi.csv
CORES="$(sysctl -n hw.logicalcpu)"
printf 'load input.csv +20000 | stage.py | save out_ril.csv\n' > rilfile
timeit "ril (auto workers)"          ./ril
printf 'load input.csv +20000 | stage.py x%s | save out_ril.csv\n' "$CORES" > rilfile
timeit "ril (x$CORES, pinned)"       ./ril
printf 'load input.csv +20000 | stage.py | save out_ril.csv\n' > rilfile

"$PY" - <<'PYEOF'
import pandas as pd
a = pd.read_csv("out_serial.csv").sort_values("id").reset_index(drop=True)
b = pd.read_csv("out_ril.csv").sort_values("id").reset_index(drop=True)
import numpy as np
ok = np.allclose(a["score"], b["score"]) and len(a) == len(b)
print(f"correctness: ril output matches serial -> {ok}  ({len(b)} rows)")
PYEOF
