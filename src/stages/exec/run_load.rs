use std::io::{BufRead, BufReader, stdout};
use std::sync::Arc;
use arrow::csv::reader::Format;
use arrow::csv::ReaderBuilder;
use arrow::ipc::writer::StreamWriter;

fn count_csv_rows(path: &str) -> anyhow::Result<usize> {
    let file = std::fs::File::open(path)?;
    let count = BufReader::new(file).lines().count();
    Ok(count.saturating_sub(1))
}

pub fn run_load(path: &str, batch_size: usize) -> anyhow::Result<()> {
    let total_rows = count_csv_rows(path)?;
    let total_batches = total_rows.div_ceil(batch_size);
    eprintln!("RIL_TOTAL_BATCHES:{total_batches}");

    let mut file = std::fs::File::open(path)?;
    let format = Format::default().with_header(true);
    let (schema, _) = format.infer_schema(&mut file, Some(100))?;

    let file = std::fs::File::open(path)?;
    let mut file_reader = ReaderBuilder::new(Arc::new(schema.clone()))
        .with_header(true)
        .with_batch_size(batch_size)
        .build(file)?;

    let mut writer = StreamWriter::try_new(stdout(), &schema)?;
    let mut batch_num = 0usize;

    while let Some(batch) = file_reader.next() {
        writer.write(&batch?)?;
        batch_num += 1;
        eprintln!("RIL_BATCH:{batch_num}");
    }
    writer.finish()?;

    Ok(())
}
