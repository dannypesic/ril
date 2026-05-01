use crate::buffer::{Receiver, BatchMessage};

pub fn run_save(path: &str, rx: Receiver) -> anyhow::Result<()> {
    use arrow::csv::WriterBuilder;

    let file = std::fs::File::create(path)?;
    let mut writer = WriterBuilder::new().build(file);

    loop {
        match rx.recv()? {
            BatchMessage::Data(batch) => writer.write(&batch)?,
            BatchMessage::Done => break,
        }
    }

    Ok(())
}  