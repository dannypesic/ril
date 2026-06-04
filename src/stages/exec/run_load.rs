use std::io::stdout;
use std::sync::Arc;
use arrow::csv::reader::Format;
use arrow::csv::ReaderBuilder;
use arrow::ipc::writer::StreamWriter;
pub fn run_load(path: &str) -> anyhow::Result<()> {

    let mut file = std::fs::File::open(path)?;
    let format = Format::default().with_header(true);
    let (schema, _) = format.infer_schema(&mut file, Some(100))?;

    let file = std::fs::File::open(path)?;
    let mut file_reader = ReaderBuilder::new(Arc::new(schema.clone()))
        .with_header(true)
        .with_batch_size(10)// Add custom sizing later.
        .build(file)?;


    let mut writer = StreamWriter::try_new(stdout(), &schema)?;

    while let Some(batch) = file_reader.next() {
        writer.write(&batch?)?;
    }
    writer.finish()?;

    Ok(())
}