from ril import rilfn
import pyarrow as pa
import pyarrow.compute as pc


@rilfn
def process(batch):
    batch = pa.record_batch(batch)
    score = pc.multiply(batch.column("value"), 2.0)
    return batch.append_column("score", score)
