# ril

Most of your CPU sits idle while one Python script grinds through a large file on a single core. ril spreads that work across all of them.

```
load data.csv | clean.py | featurize.py | save output.csv
```

ril is for slow, CPU-bound Python that runs row by row or batch by batch, which can't be vectorized away and only gets slower as the data grows. Usually it's slow because it does real work on every record, on one core, while the rest of the machine sits idle. ril takes the steps you've already written and runs them across all your cores at once. It's a single binary that uses the machine you already have without forcing parallelism code.

Each stage is a Python function that takes a batch of records and returns a transformed batch. Under the hood, ril runs your stages as separate processes, streams Apache Arrow RecordBatches between them, and within each stage splits the batches across workers and reassembles the results in order.

Peak memory stays flat whether the file is 500MB or 50GB, because data moves through in chunks instead of loading all at once. While benchmarking, a CPU-bound pass over a one-million-row CSV drops from 47s in single-core pandas to about 12s, at a fraction of the memory.

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

## When to use ril

ril is for CPU-bound Python that runs per row or per batch and can't be easily vectorized away, especially when it spans more than one step. If a transform is slow because it does real Python work on every record, and you want that work spread across cores, that is the case ril is built for. It fits work where the goal is the output rather than the infrastructure, such as scientific computing, simulations, research code, and one-off data jobs. In those settings you want a slow transform to finish faster without adopting a parallelism framework or standing up a cluster to get there.

For DataFrame-style transforms that fit Polars or DuckDB's expression API, ril is not the better choice; those engines will be faster and simpler. ril earns its place when you have a multi-stage pipeline, non-Polars code, or CPU-bound Python that needs true multi-core execution across stages. They aren't mutually exclusive: Polars is a natural choice inside a ril stage.

`multiprocessing.Pool` gets you the same multi-core speedup (see [Benchmarks](#benchmarks)), but you write and maintain the Pool, the IO, and the chunking yourself, and the whole dataset sits in memory. ril handles the parallelism and IO and streams the data through instead of buffering it.

Ray, Dask, and Spark are built for clusters. When your data fits on one machine and you just want to use all of its cores, ril does that with a single binary and no infrastructure.

## Benchmarks

Applying a CPU-bound Python function to every row of a one-million-row CSV, on an 8-core laptop:

| variant              | time   |
|----------------------|--------|
| single-core (pandas) | 47.5 s |
| multiprocessing.Pool | 11.8 s |
| ril (auto workers)   | 16.1 s |
| ril (`x8`, pinned)   | 11.9 s |

With workers pinned, ril matches a hand-tuned `multiprocessing.Pool` to within a few percent, and gets there without the Pool boilerplate or the manual IO. That works out to nearly 4x over single-core pandas on this machine; `Pool` reaches the same ceiling, so it's the limit of the workload and the hardware rather than of ril. Peak memory also stays flat as the input grows, holding around 160MB at every size in this test while the pandas approaches climb past 700MB.

The gap between the two ril rows comes down to how you configure the pipeline; see [Tuning](#tuning). The full harness and the memory-scaling numbers are in [benchmarks/](benchmarks/).

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

ril allocates workers automatically. It profiles the first few batches of each untagged script stage (taking a trimmed mean, dropping the slowest and fastest), then divides your CPU cores among the stages in proportion to how long each one takes, so a slower stage gets more workers. `load`, `save`, and `tee` are I/O-bound and always run as a single process.

To pin a stage to a fixed worker count, tag it with `xN`. A pinned stage gets exactly N workers and is left out of the automatic split; the remaining cores are shared among the untagged stages.

```
load data.csv | clean.py | featurize.py    | save output.csv   # both auto-allocated
load data.csv | clean.py | featurize.py x4 | save output.csv   # featurize pinned to 4, clean auto
```

Pinning also skips the profiling step, so it's worth doing for a stage you already know is the bottleneck (see [Tuning](#tuning)).

### Tuning

Two settings decide how well a pipeline parallelizes.

Batch size is set on `load`, for example `load data.csv +20000`. ril splits each batch across the workers in a stage, so batches that are too large parallelize coarsely, while batches that are too small spend more time on per-batch overhead than on your code. A few thousand to a few tens of thousands of rows is a reasonable place to start; adjust it for your row width and per-row cost. This is the same kind of choice you would make for a `multiprocessing.Pool` chunksize.

Worker placement is the other setting. Auto mode profiles the first five batches of each untagged stage on a single worker, then scales up and runs as fast as a pinned pipeline from there. Profiling is per stage, so pinning a stage with `xN` skips its warm-up, and a pipeline with every stage pinned does none. The warm-up is a one-time cost that only stands out when few batches follow it. Pin for a small run of a few batches; for a large dataset the cost amortizes to nothing, and since auto balances workers across stages for you, leaving stages untagged is the better default there. The two ril rows in the benchmark above are a single-stage, mid-size run where the warm-up is still visible.

How much any of this helps depends on the work itself. A stage doing heavy per-row Python scales close to your core count, while a light stage is limited by IO and parsing, where extra workers do little.

### Failure semantics

ril is fail-fast. An error in any stage aborts the whole pipeline and exits non-zero. It does not retry, skip, or resume for data integrity.

When your `@rilfn` raises, ril prints the stage and the batch index along with the full Python traceback:

```
ril error: stage 1 (featurize.py)

batch 3:
Traceback (most recent call last):
  File "featurize.py", line 10, in process
    raise ValueError("kaboom")
ValueError: kaboom
```

A few things to know before you try it:

- Output is not transactional. Stages stream, so by the time a later batch fails, `save` and `tee` may have already written every batch that came before it. ril does not roll those back, so treat an output file as valid only when the run exits `0`.
- The schema must be identical across batches. The output schema is fixed by the first batch a stage emits, and ril checks every later batch against it. A batch that returns different columns or types stops the stage with an error naming the batch and both schemas. Return the same columns and types from every call.
- Only ordinary exceptions carry a traceback. A hard crash such as a segfault, the OOM killer, or `os._exit` can't report one, so you get a `worker failed` and a non-zero exit instead.
- There is no timeout. A stage that hangs, whether on an infinite loop or a blocking call, hangs the pipeline. Nothing kills it for you.

### Compatibility

ril connects to `pip` or `uv` in your project to manage the `.venv`:

- `pip`: default
- `uv`: used automatically if a `uv.lock` file is detected

On startup, ril auto-detects the newest compatible Python interpreter available on your PATH, trying `python3.14`, `python3.13`, `python3.12`, `python3.11`, then `python3` as a fallback.

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
