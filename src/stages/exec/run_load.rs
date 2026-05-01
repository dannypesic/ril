use crate::buffer::{Sender, BatchMessage};

pub fn run_load(path: &str, tx: Sender) -> anyhow::Result<()> {
    use arrow::csv::{ReaderBuilder, reader::Format};
    use std::sync::Arc;

    let mut file = std::fs::File::open(path)?;

    // Infer column types from first 100 rows
    let format = Format::default().with_header(true);
    let (schema, _) = format.infer_schema(&mut file, Some(100))?;

    // Rewind and build a batched reader (1000 rows per batch)
    let file = std::fs::File::open(path)?;
    let mut reader = ReaderBuilder::new(Arc::new(schema))
        .with_header(true)
        .with_batch_size(1000)
        .build(file)?;

    while let Some(batch) = reader.next() {
        tx.send(BatchMessage::Data(batch?))?;
    }

    tx.send(BatchMessage::Done)?;
    Ok(())
}