use arrow::record_batch::RecordBatch;

pub type Batch = RecordBatch;

// Messages sent through the channel between stages.
// Done signals the downstream stage to stop reading.
pub enum BatchMessage {
    Data(Batch),
    Done,
}

// The channel type. Bounded at 8 batches — when full, upstream blocks.
// This is your backpressure. crossbeam's bounded channel blocks the sender
// when the queue is full rather than growing unboundedly.
pub type Sender = crossbeam_channel::Sender<BatchMessage>;
pub type Receiver = crossbeam_channel::Receiver<BatchMessage>;

pub fn channel() -> (Sender, Receiver) {
    crossbeam_channel::bounded(8)
}