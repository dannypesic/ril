# Routed Interpreter Layer

Parallel pipeline executor that streams Apache Arrow RecordBatches between stages. Write Python transform scripts, chain them with `|`, and ril handles the data flow.

```
load data.csv | clean.py | save output.csv
```

Run with `ril` in the directory containing your `rilfile`.

### Compatibility

ril connects directly with `pip` or `uv` in your project to ensure clean integration:
- `pip`: default
- `uv`: if `uv.lock` detected

ril is currently locked to Python 3.14.

## How it works

Each stage runs as its own process, receiving Arrow RecordBatches from the previous stage and forwarding results to the next. Stages run concurrently with backpressure automatic via Unix pipes.

Within each stage, ril spawns multiple worker processes and splits each incoming batch across them: each worker gets a slice, processes it in parallel, and the results are reassembled in order. This sidesteps the GIL entirely since each interpreter runs independently, and delivers close to linear speedup up to your core count (~8× on an 8-core CPU).

Data streams through in chunks (1000 rows by default, configurable with `+N` on `load`) rather than loading everything at once, so memory usage stays constant regardless of file size. The `@rilfn` function is called once per chunk, which means operations that need the full dataset (e.g. a global sort) don't belong inside a single stage.

### Writing a stage

```python
from ril import rilfn
import pyarrow as pa

@rilfn
def process(batch):
    batch = pa.record_batch(batch)
    data = batch.to_pydict()
    data["sum"] = [a + b for a, b in zip(data["value1"], data["value2"])]
    return pa.RecordBatch.from_pydict(data)
```

Works with pandas/numpy as well via PyArrow.

## Built-in stages

| Stage  | Example                   | Notes                                      |
|--------|---------------------------|--------------------------------------------|
| `load` | `load data.csv`           | streams in batches of 1000 rows by default |
| `load` | `load data.csv +500`      | custom batch size (rows per chunk)         |
| `save` | `save output.csv`         |                                            |
| `tee`  | `tee checkpoint.csv`      |                                            |

### Worker count

By default, ril runs in dynamic mode: it profiles the first 5 batches per script stage (dropping the min and max, averaging the rest), then allocates workers proportionally to equalize throughput across stages, up to your CPU core count. Load, save, and tee are excluded since they're I/O-bound.

If any stage has an explicit worker tag, dynamic mode is disabled for the whole pipeline — untagged stages get 1 worker, tagged stages get exactly N:

```
load data.csv | model.py x4 | save output.csv   # fixed mode: model.py gets 4 workers
load data.csv | model.py xD | save output.csv   # fixed mode: one worker per CPU core
load data.csv | model.py    | save output.csv   # dynamic mode: workers auto-allocated
```

## Building

```bash
cargo build --release
```

Requires Rust and Python 3.14. On first run, ril automatically creates a `.venv` and `ril.py` in your project directory and installs `pyarrow` and `arro3-core`.

## Current limitations

Limited error handling.

### Next

- More advanced error handling
- Live progress display
- Proper documentation
