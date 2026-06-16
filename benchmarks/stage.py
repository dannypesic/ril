from ril import rilfn
import pyarrow as pa
from workload import heavy


@rilfn
def process(batch):
    batch = pa.record_batch(batch)
    d = batch.to_pydict()
    d["score"] = [heavy(v) for v in d["value"]]
    return pa.RecordBatch.from_pydict(d)
