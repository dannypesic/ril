# ril

Your Python scripts run while most of your CPU is sleeping. ril fixes that.

```
load data.csv | clean.py | featurize.py x4 | save output.csv
```

ril streams Apache Arrow RecordBatches between stages. Each stage is a separate Python process, meaning no GIL contention, no threads, and no rewriting your code for `multiprocessing`. Your functions just transform data; ril handles splitting, parallelism, and reassembly.

Data streams through in chunks, so memory stays constant regardless of file size. A 50GB CSV uses the same peak memory as a 500MB one.

Built for data engineers, ML practitioners, and researchers who write Python transforms on large datasets and are tired of waiting for single-core pandas or spinning up distributed infrastructure to parallelize a handful of scripts.

**A rilfile:**

```
load data.csv +2000
| clean.py
| tee cleaned_data.csv
| featurize.py
| score.py
| save output.csv
```

**A stage:**

```python
from ril import rilfn
import pyarrow as pa

@rilfn
def process(batch):
    batch = pa.record_batch(batch)
    data = batch.to_pydict()
    data["score"] = [x * 2.0 for x in data["value"]]
    return pa.RecordBatch.from_pydict(data)
```

ril calls your function once per chunk, passes a `pyarrow.RecordBatch`, and expects one back. Works natively with pandas, numpy, and any other module that supports PyArrow.

---

## Documentation

### How it works

Each stage runs as its own process. Stages receive Arrow RecordBatches from the previous stage over a Unix pipe and forward results to the next. Stages run concurrently; backpressure is automatic via the pipe buffer.

Within each script stage, ril spawns multiple worker processes and splits each incoming batch across them. Each worker gets a chunk, processes it independently, and the results are reassembled in order. Because each worker is a separate interpreter, there is no shared GIL, so all cores run consistently. Memory usage stays constant regardless of file size since data streams through in chunks rather than loading all at once.

The `@rilfn` function is called once per chunk. Operations that require the full dataset (e.g. a global sort) do not belong inside a single stage.

### What about free-threaded Python?

Free-threaded builds (3.13t, 3.14t) remove the GIL, but threads still share memory, meaning shared reference counts, Python object mutations, and C extensions that weren't written with thread safety in mind. Most of the data stack (numpy, pandas, and most ML libraries) isn't fully thread-safe yet, so you'd need to audit every dependency before trusting a threaded pipeline.

ril uses separate processes. Each worker has its own interpreter and memory space, so none of that applies; your existing scripts work as-is. If you do run a free-threaded build, ril still works fine; you just get extra headroom within each worker on top of the parallelism across them.

### Writing a stage

Create a `.py` file with a function decorated with `@rilfn`:

```python
from ril import rilfn
import pyarrow.compute as pc

@rilfn
def process(batch):
    # batch is a pyarrow RecordBatch
    mask = pc.greater(batch.column("value"), 0)
    return batch.filter(mask)
```

Place the file in your project directory and reference it by name in the rilfile. ril calls the decorated function once per chunk with a `pyarrow.RecordBatch` and expects a `pyarrow.RecordBatch` back.

### Built-in stages

| Stage  | Example                   | Notes                                      |
|--------|---------------------------|--------------------------------------------|
| `load` | `load data.csv`           | streams in batches of 1000 rows by default |
| `load` | `load data.csv +500`      | custom batch size (rows per chunk)         |
| `save` | `save output.csv`         | terminal stage, writes final output                        |
| `tee`  | `tee checkpoint.csv`      | writes to file and passes batches through  |

### Worker count

By default, ril runs in dynamic mode: it profiles the first 5 batches per script stage (dropping the min and max, averaging the rest), then allocates workers proportionally to equalize throughput across stages, up to your CPU core count. `load`, `save`, and `tee` are excluded since they are I/O-bound.

If any stage has an explicit worker tag, fixed mode applies to the whole pipeline: untagged stages get 1 worker, tagged stages get exactly N:

```
load data.csv | model.py x4 | save output.csv   # fixed: model.py gets 4 workers
load data.csv | model.py xD | save output.csv   # fixed: one worker per CPU core
load data.csv | model.py    | save output.csv   # dynamic: workers auto-allocated
```

### Compatibility

ril connects to `pip` or `uv` in your project to manage the `.venv`:

- `pip`: default
- `uv`: used automatically if a `uv.lock` file is detected

On startup, ril auto-detects the newest compatible Python interpreter available on your PATH, trying `python3.14`, `python3.13`, `python3.12`, `python3.11`, then `python3` as a fallback.

### Building

```bash
cargo build --release
```

Requires Rust and Python 3.11–3.14. On first run, ril automatically creates a `.venv` and `ril.py` in your project directory and installs `pyarrow` and `arro3-core`.

To build against a specific Python version, set `PYO3_PYTHON` before running cargo:

```bash
PYO3_PYTHON=python3.12 cargo build --release
```

`PYO3_PYTHON` controls which Python interpreter the binary links against at compile time. Pre-built release binaries are compiled per Python version, found in the Github releases page. Download the asset matching your Python install.
