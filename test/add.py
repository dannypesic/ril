from ril import rilfn
import pyarrow as pa
import pyarrow.compute as pc

@rilfn
def process(batch):
    print("Python Running!")
    batch = pa.record_batch(batch)

    data = batch.to_pydict()
    data["sum"] = []
    for i in range(len(data["value1"])):
        data["sum"].append(data["value1"][i] + data["value2"][i])
    batch = pa.RecordBatch.from_pydict(data)
    return batch
