use std::path::PathBuf;
use std::sync::mpsc;
use arrow::array::RecordBatch;
use arrow::compute::concat_batches;
use crate::stages::worker::Worker;

fn split_record_batch(batch: RecordBatch, n: usize) -> Vec<RecordBatch> {
    assert!(n > 0);

    let total_rows = batch.num_rows();

    let base_size = total_rows / n;
    let remainder = total_rows % n;

    let mut offset = 0;
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let size = base_size + if i < remainder { 1 } else { 0 };

        let sliced = batch.slice(offset, size);
        out.push(sliced);

        offset += size;
    }

    out
}

pub struct Manager {
    pub(crate) count_workers: usize,
    workers: Vec<Worker>,
    result_rx: mpsc::Receiver<(usize, RecordBatch)>
}
impl Manager {
    pub fn new(exe: PathBuf, count_workers: usize, flags: Vec<(String, String)>, stage_index: usize, is_python: bool) -> anyhow::Result<Self> {
        let (result_tx, result_rx) = mpsc::channel();
        let mut workers = Vec::with_capacity(count_workers);
        for idx in 0..count_workers {
            workers.push(Worker::new(&exe, idx, &flags, result_tx.clone(), stage_index, is_python)?)
        }
        Ok(Self {
            count_workers,
            workers,
            result_rx,
        })
    }

    pub fn kill(self) -> anyhow::Result<()> {
        for mut worker in self.workers {
            drop(worker.tx);
            worker.thread.join().expect("Worker thread panicked");
            worker.process.wait()?;
        }
        Ok(())
    }

    pub fn run_thread_work(&mut self, batch: RecordBatch) -> anyhow::Result<RecordBatch> {

        let sub_batches = split_record_batch(batch, self.count_workers);

        for (idx, sub_batch) in sub_batches.into_iter().enumerate() {
            self.workers[idx].tx.send(sub_batch)?;
        }

        let mut results: Vec<Option<RecordBatch>> = vec![None; self.count_workers];

        for _ in 0..self.count_workers {
            let (idx, batch) = self.result_rx.recv()?;
            results[idx] = Some(batch);
        }
        let schema = results[0].as_ref().unwrap().schema();

        Ok(concat_batches(&schema, &results.into_iter().map(|b| b.unwrap()).collect::<Vec<_>>())?)
    }

}