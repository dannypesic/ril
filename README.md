# Routed Interpreter Layer

Routed Interpreter Layer is a pipeline executor that streams Apache Arrow RecordBatches between stages. Write Python transform scripts, chain them with `|`, and ril handles the data flow.

```bash
ril "load data.csv | clean.py | save output.csv"
ril pipeline.ril
```

## How it works

Each stage in the pipeline receives Arrow RecordBatches from the previous stage and sends batches to the next. Stages run concurrently with bounded channels between them so backpressure is automatic.

A script tagged with `@rilfn` runs **in-process** via an embedded Python interpreter (no subprocess overhead, no serialization). 

### Batches and chunks

ril doesn't load your data all at once, rather, it streams it through the pipeline in chunks of 1000 rows at a time. Your `@rilfn` function is called once per chunk, not once per file. This means you can process files larger than memory, but it also means you shouldn't do anything that requires seeing all rows at once (e.g. sorting across the full dataset) inside a single stage.

### Working with data

You can work directly with the PyArrow API, or convert to Python dicts for row-level logic:

```python
@rilfn
def process(batch):
    batch = pa.record_batch(batch)

    # convert to dict of lists — easy for row-level operations
    data = batch.to_pydict()
    data["sum"] = [a + b for a, b in zip(data["value1"], data["value2"])]

    return pa.RecordBatch.from_pydict(data)
```

You can also work through pandas or numpy for your data pipelines:

```python
import pandas as pd

@rilfn
def process(batch):
    df = batch.to_pandas()
    df["sum"] = df["value1"] + df["value2"]
    return pa.RecordBatch.from_pandas(df)
```

## Built-in stages

| Stage | Example |
|---|---|
| `load` | `load data.csv` |
| `save` | `save output.csv` |
| `filter` | `filter 'value > 0'` |
| `select` | `select name age value` |
| `each` | `each { clean.py }` |

## .ril files

You can write your pipeline in a `.ril` file and pass it directly to ril instead of quoting the pipeline inline:

```
# pipeline.ril
load data.csv | clean.py | save output.csv
```

```bash
ril pipeline.ril
```

To try a working example, run the test pipeline from the repo root:

```bash
ril test/script.ril
```

This loads `test/data.csv`, runs `test/add.py` (which adds two columns together), and writes the result to `test/results.csv`.

## Current limitations

The executor is **thread-based**, which means all `@rilfn` stages share a single Python interpreter (GIL applies). This works fine for linear pipelines but means you can only have one active Python instance at a time. Process-based execution is in progress so each stage will run in its own interpreter with Arrow IPC passed between them.

## Building

```bash
cargo build --release
```

Requires Rust and a Python installation with `pyarrow` and `arro3` available.
