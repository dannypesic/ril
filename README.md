# Routed Interpreter Layer

Routed Interpreter Layer is a pipeline executor that streams Apache Arrow RecordBatches between stages. Write Python transform scripts, chain them with `|`, and ril handles the data flow.

#### rilfile:
```
load data.csv | clean.py | save output.csv
```
Then simply run `ril`.

### Compatability

ril connects directly with `pip` or `uv` in your project to ensure clean integration:
- `pip`: default
- `uv`: if `uv.lock` detected

ril is currently locked to Python 3.14

## How it works

Each stage in the pipeline receives Arrow RecordBatches from the previous stage and sends batches to the next. Stages run concurrently with bounded channels between them so backpressure is automatic.

A script with a function tagged `@rilfn` runs in-process via an embedded Python interpreter (no subprocess overhead, no serialization). 

### Batches and chunks

ril streams data through the pipeline in chunks of 1000 rows at a time rather than loading it all at once. The `@rilfn` function is called once per chunk rather than once per file. This means you can process files larger than memory, but it also means you shouldn't do anything that requires seeing all rows at once (e.g. sorting across the full dataset) inside a single stage.

### Working with data

You can work directly with the PyArrow API, or convert to Python dicts for row-level logic:

```python
from ril import rilfn
import pyarrow as pa

@rilfn
def process(batch):
    batch = pa.record_batch(batch)

    # convert to dict of lists for easy row-level operations
    data = batch.to_pydict()
    data["sum"] = [a + b for a, b in zip(data["value1"], data["value2"])]

    return pa.RecordBatch.from_pydict(data)
```

You can also work through pandas or numpy for your data pipelines:

```python
import pandas as pd
import pyarrow as pa
from ril import rilfn

@rilfn
def process(batch):
    df = batch.to_pandas()
    df["sum"] = df["value1"] + df["value2"]
    return pa.RecordBatch.from_pandas(df)
```

## Built-in stages

| Stage  | Example              |
|--------|----------------------|
| `load` | `load data.csv`      |
| `save` | `save output.csv`    |
| `tee`  | `tee checkpoint.csv` |
| `xN`   | `model.py x4`        |
| `xD`   | `model.py xD`        |

`xN` and `xD` currently WIP.

## Multiprocessing

Each stage spawns a child process to independently compute data and stream it through Arrow IPC via stdio. This allows for parallel processing of data while avoiding Python's Global Interpreter Lock, as each interpreter runs independently.

## rilfile

You can write your pipeline in a file named `rilfile`. Calling `ril` in your shell in that directory executes the pipeline.

```rilfile
load data.csv | clean.py | save output.csv
```

```bash
ril pipeline.ril
```

To try a working example, run the test pipeline via `ril` from the repo root.

This loads `test/data.csv`, stream through `test/add.py` (which adds two columns together), checkpoints with `tee` to `test.check.csv`, streams through `test/diff.py` (which takes their difference), then writes the result to `test/results.csv`.

## Current limitations

Currently, ril lacks significant error handling along with parallel compute within processes.

### Next

- More advanced error handling systems
- Subprocesses manage workers for easy and intuitive parallel compute (`test.py x3`)
- Dynamic worker process allocation with `xD`
- Live progress display
- Chunk size customizability

## Building

```bash
cargo build --release
```

Requires Rust and Python 3.14. On first run, ril will automatically create a `.venv` and `ril.py` in your project directory and install `pyarrow` and `arro3-core`, so no manual environment setup is needed.

