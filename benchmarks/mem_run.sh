#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

PY=".venv/bin/python"
[ -d .venv ] || { echo "run ./run.sh first to init .venv"; exit 1; }
cp ../target/release/ril ./ril

printf 'load input.csv +20000 | stage_light.py x1 | save out_ril.csv\n' > rilfile

for ROWS in 2000000 8000000 16000000; do
  "$PY" gen_data.py "$ROWS" input.csv >/dev/null
  sz=$(du -h input.csv | cut -f1)
  echo "rows=$ROWS  (input.csv $sz)"
  "$PY" mem_bench.py "  pandas (whole-file)" -- "$PY" mem_serial.py input.csv out_serial.csv
  "$PY" mem_bench.py "  ril (streaming)"     -- ./ril
  echo
done

printf 'load input.csv +20000 | stage.py | save out_ril.csv\n' > rilfile
