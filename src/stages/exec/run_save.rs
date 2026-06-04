use std::io::stdin;
use arrow::csv::WriterBuilder;
use arrow::ipc::reader::StreamReader;

pub fn run_save(path: &str) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = WriterBuilder::new().build(file);
    let mut reader = StreamReader::try_new(stdin(), None)?;
    let mut batch_num = 0usize;

    for batch in &mut reader {
        let batch = batch?;
        writer.write(&batch)?;
        batch_num += 1;
        eprintln!("RIL_BATCH:{batch_num}");
    }

    Ok(())
}
