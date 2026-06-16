# ril

Most of your CPU sits idle while one Python script grinds through a large file on a single core. ril spreads that work across all of them.

```
load data.csv | clean.py | featurize.py | save output.csv
```

ril is for slow, CPU bound Python that iterates through a dataset. It spawns worker processes that each run a Python interpreter and transform Arrow RecordBatches while streaming to each other. It automatically divides batches between workers and reassembles them in order. Since data is streamed, peak memory usage stays constant even as dataset size rises. It works on arbitrary Python scripts that take in a `pyarrow.RecordBatch` and return the processed one. 

It is signficiantly faster than single core pandas and matches a `multiprocessing.Pool` (see [Benchmarks](#benchmarks)), but memory streams by default and it doesn't require rewriting for a Pool or managing IO.

ril works well on large datasets and complex transforms. It fits work where fast and efficient output is prioritized over setting up infrastructure, such as simulations, research code, scientific computing, and one-off data jobs.

A rilfile:

```
load data.csv +2000
| clean.py
| tee cleaned_data.csv
| featurize.py
| score.py
| save output.csv
```

A stage:

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

It works natively with pandas, numpy, and any other module that supports PyArrow.

## Benchmarks

Applying a CPU bound Python function to every row of a one million row CSV:

| variant              | time   |
|----------------------|--------|
| single-core (pandas) | 47.5 s |
| multiprocessing.Pool | 11.8 s |
| ril (auto workers)   | 16.1 s |
| ril (`x8`, pinned)   | 11.9 s |

With workers pinned, ril matches a manually tuned `multiprocessing.Pool` to within a few percent, and works without configuring a custom `Pool`. It works out to nearly 4x over single-core pandas on this machine, reaching the same ceiling as `Pool`. Peak memory also stays flat as the input grows, holding around 160MB at every size in this test while the pandas approaches climb past 700MB.

The gap between the two ril rows comes down to how you configure the pipeline; see [Tuning](#tuning). The full harness and the memory-scaling numbers are in [benchmarks/](benchmarks/).

## Versus Other Frameworks

For DataFrame transforms that fit Polars or DuckDB's expression API, those engines will be faster and simpler. ril is effective for arbitrary Python that can't be purely experessed with relation logic. They aren't mutually exclusive: Polars is a natural choice inside a ril stage.

`multiprocessing.Pool` gives the same multi-core speedup (see [Benchmarks](#benchmarks)), but requires more technical overhead and doesn't stream data. ril handles the parallelism and streams data through rather than buffering it.

Ray, Dask, and Spark are built for clusters. ril gives speedups with a single binary and no infrastructure.

---

## Documentation

### How it works

Each stage runs as its own process. Stages receive Arrow RecordBatches from the previous stage over a Unix pipe and forward results to the next. Stages run concurrently, and backpressure is automatic via the pipe buffer.

The `@rilfn` function is called once per chunk. Operations that require the full dataset (e.g. a global sort) do not belong inside a single stage.

### Free-threaded Python

Free-threaded builds (3.13t, 3.14t) remove the GIL, but threads still share memory. Most of the data stack (numpy, pandas, and most ML libraries) isn't fully thread-safe, so you need to audit every dependency before using a threaded pipeline.

Since ril uses separate processes, each worker has its own interpreter and memory space, so none of that applies: your existing scripts work as they are. ril still works on free-threaded builds, giving extra headroom within each worker on top of the parallelism across them.

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

### Binary stages

Any executable that reads Arrow IPC from stdin and writes Arrow IPC to stdout works as a stage. Reference it by path:

```
load data.csv | ./transform | save output.csv
```

The binary receives batches one at a time and must flush stdout after writing each result. Worker count tags work the same as for Python stages.

### Built-in stages

| Stage  | Example                   | Notes                                      |
|--------|---------------------------|--------------------------------------------|
| `load` | `load data.csv`           | streams in batches of 1000 rows by default |
| `load` | `load data.csv +500`      | custom batch size (rows per chunk)         |
| `save` | `save output.csv`         | terminal stage, writes final output        |
| `tee`  | `tee checkpoint.csv`      | writes to file and passes batches through  |

### Worker count

ril allocates workers automatically. It profiles the first few batches of each untagged script stage (dropping the slowest and fastest then taking the mean), then divides your CPU cores among the stages in proportion to how long each one takes, so a slower stage gets more workers. `load`, `save`, and `tee` are I/O-bound and always run as a single process.

To pin a stage to a fixed worker count, tag it with `xN`. A pinned stage gets exactly N workers and is left out of the automatic split; the remaining cores are shared among the untagged stages.

```
load data.csv | clean.py | featurize.py    | save output.csv   # both auto-allocated
load data.csv | clean.py | featurize.py x4 | save output.csv   # featurize pinned to 4, clean auto
```

Pinning all stages skips the profiling step, which removes the allocation startup on small datasets.

### Tuning

Two settings decide how well a pipeline parallelizes.

Batch size is set on `load`, for example `load data.csv +20000`. ril splits each batch across the workers in a stage, so large batches parallelize coarsely while small batches spend more time on per-batch overhead rather than on your code. A few thousand to a few tens of thousands of rows is a reasonable place to start, which can be adjusted for your row width and per-row cost. This is the same kind of choice you would make for a `multiprocessing.Pool` chunksize.

The other setting is worker placement. Auto mode profiles the first five batches on each untagged stage on one worker, then scales up and runs as fast as a pinned pipeline from there. The profile is a one time cost that becomes negligible on larger datasets with reasonable batch sizes, and mostly becomes faster given the improved efficiency. It is recommended to use auto mode except on a dataset with few batches, as the overhead can cause a significant slowdown. 

### Failure semantics

ril is fail fast: an error in any stage aborts the whole pipeline and exits non-zero. It does not retry, skip, or resume for data integrity.

When your `@rilfn` raises, ril prints the stage and the batch index along with the full Python traceback:

```
ril error: stage 1 (featurize.py)

batch 3:
Traceback (most recent call last):
  File "featurize.py", line 10, in process
    raise ValueError("kaboom")
ValueError: kaboom
```

A few things to know before trying:

- Output is not transactional. Stages stream, so by the time a later batch fails, `save` and `tee` may have already written every batch that came before it. ril does not roll those back, so treat an output file as valid only when the run exits `0`.
- The schema must be identical across batches. The output schema is fixed by the first batch a stage emits, and ril checks every later batch against it. A batch that returns different columns or types stops the stage with an error naming the batch and both schemas. Return the same columns and types from every call.
- Only ordinary exceptions carry a traceback. A hard crash such as a segfault, the OOM killer, or `os._exit` can't report one, so you get a `worker failed` and a non-zero exit instead.
- There is no timeout by default. A stage that hangs, whether on an infinite loop or a blocking call, hangs the pipeline.

### Compatibility

ril connects to `pip` or `uv` in your project to manage the `.venv`:

- `pip`: default
- `uv`: used automatically if a `uv.lock` file is detected

On startup, ril automatically detects the newest compatible Python interpreter available on your PATH, trying `python3.14`, `python3.13`, `python3.12`, `python3.11`, then `python3` as a fallback.

### Feedback

Bug reports and feature requests are welcome. Open an issue on GitHub.

### Building

```bash
cargo build --release
```

Requires Rust and Python 3.11-3.14. On first run, ril automatically creates a `.venv` and `ril.py` in your project directory and installs `pyarrow` and `arro3-core`.

To build against a specific Python version, set `PYO3_PYTHON` before running cargo:

```bash
PYO3_PYTHON=python3.12 cargo build --release
```

`PYO3_PYTHON` controls which Python interpreter the binary links against at compile time. Pre-built release binaries are compiled per Python version, found in the Github releases page. Download the asset matching your Python install.
