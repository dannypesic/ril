use std::io::{stdin, stdout};
use std::io::Stdout;
use arrow::csv::WriterBuilder;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;

pub fn run_tee(path: &str) -> anyhow::Result<()> {

    let file = std::fs::File::create(path)?;
    let mut file_writer = WriterBuilder::new().build(file);

    let mut reader = StreamReader::try_new(stdin(), None)?;
    let mut writer: Option<StreamWriter<Stdout>> = None;
    
    for batch in &mut reader {
        let batch = batch?;
        file_writer.write(&batch)?;
        let w = match writer {
            Some(ref mut w) => w,
            None => {
                writer = Some(StreamWriter::try_new(stdout(), batch.schema_ref())?);
                writer.as_mut().unwrap()
            }
        };
        w.write(&batch)?;
    }
    if let Some(mut w) = writer {
        w.finish()?;
    }
    Ok(())

}

