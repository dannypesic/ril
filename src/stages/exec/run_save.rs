use std::io::stdin;
use arrow::csv::WriterBuilder;
use arrow::ipc::reader::StreamReader;

pub fn run_save(path: &str) -> anyhow::Result<()> {

    let file = std::fs::File::create(path)?;
    let mut writer = WriterBuilder::new().build(file);

    let mut reader = StreamReader::try_new(stdin(), None)?;
    loop {
        for batch in reader.next() { // do what this says
            let batch = batch?;
            writer.write(&batch)?
        }
        if reader.is_finished() { break }
    }

    Ok(())
}