from ril import rilfn
import pyarrow as pa
import pyarrow.compute as pc

@rilfn
def process(batch):
    batch = pa.record_batch(batch)

    data = batch.to_pydict()
    data["diff"] = []
    for i in range(len(data["value1"])):
        data["diff"].append(data["value2"][i] - data["value1"][i])
    batch = pa.RecordBatch.from_pydict(data)
    return batch
