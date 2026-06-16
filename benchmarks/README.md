# Benchmarks

Reproducible benchmarks comparing ril against single-threaded code (pandas), or setting up `multiprocessing.Pool`.

## What's measured

An identical CPU-bound, per-row Python transform (`workload.heavy`) applied to every row of a CSV, run three ways with the exact same function:

- single-core (pandas): `serial.py`, the initial case.
- multiprocessing.Pool: `multi.py`, the manual parallel baseline, structured around the Pool with no default data streaming.
- ril: `stage.py`, just the transform. Requires no I/O code or Pool setup.

`heavy` is deliberately not vectorizable; it is a pure Python loop per row. That is the case ril targets. If your transform is expressible in Polars/DuckDB they will beat all of the above. ril is for arbitrary Python you have to actually run.

## Running

```bash
./run.sh           # throughput, 1,000,000 rows (override: ./run.sh 5000000)
./mem_run.sh       # constant memory demo across growing input sizes
```

`run.sh` bootstraps a `.venv` on first run and installs pandas.

## Results

Apple M1 (4 performance + 4 efficiency cores), 8 GB, macOS. 1,000,000 rows. Wall clock is stable to ~1% across runs.

### Throughput: compute-bound transform (`K=400`)

| variant              | time   | vs single-core |
|----------------------|--------|----------------|
| single-core (pandas) | 47.5 s | 1.0×           |
| multiprocessing.Pool | 11.8 s | 4.0×           |
| ril (auto workers)   | 16.1 s | 3.0×           |
| ril (`x8`, pinned)   | 11.9 s | 4.0×           |

Analysis notes:

- ~4× is the ceiling rather than 8x. The M1 has 4 performance cores; the 4 efficiency cores contribute little. `multiprocessing.Pool` hits the same wall.
- ril with an explicit worker tag matches hand-tuned `Pool` (11.8 s vs 11.9 s), with none of the Pool/IO boilerplate.
- Auto mode is slower on heavy stages (16.1 s). Auto mode profiles the first 5 batches with a single worker before scaling up; when each batch is expensive that warm-up costs real time. Pin stages with an explicit `xN` tag to skip the warm-up. On large datasets, this performance dip is effectively negligible.

### Throughput: lighter transform (`K=60`)

| variant                 | time    |
|-------------------------|---------|
| single-core (pandas)    | 8.4 s   |
| multiprocessing.Pool    | 3.7 s   |
| ril (auto workers)      | 3.65 s  |

When per-row work is light, CSV parsing/writing dominates and all parallel approaches converge on the same floor; ril's auto-profiling overhead disappears.

### Constant memory (`mem_run.sh`)

Peak RSS of the whole process tree for a light transform as input grows. pandas loads the whole file; ril streams it in batches.

| rows | input  | pandas (whole-file) | ril (streaming) |
|------|--------|---------------------|-----------------|
| 2M   | 28 MB  | 202 MB              | 149 MB          |
| 8M   | 128 MB | 426 MB              | 157 MB          |
| 16M  | 240 MB | 732 MB              | 162 MB          |

ril's footprint is flat regardless of file size, while the pandas path grows with it. This is the property that lets a 50 GB file use the same memory as a 500 MB one.

## Measurement notes

- `mem_bench.py` samples summed RSS across the process tree every 30 ms. Summing RSS overcounts shared library pages (each worker counts its mapped copy of libpython/pyarrow), so absolute multi-process numbers in `run.sh` run high. The constant memory table is the trustworthy memory result because the shared page offset is roughly constant across input sizes. The trend is what matters and is robust to it.