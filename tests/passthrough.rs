use std::io::{stdin, stdout, Write};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;

fn main() {
    let mut reader = StreamReader::try_new(stdin(), None).unwrap();
    let mut writer: Option<StreamWriter<_>> = None;
    for batch in &mut reader {
        let batch = batch.unwrap();
        let w = match writer {
            Some(ref mut w) => w,
            None => {
                writer = Some(StreamWriter::try_new(stdout(), batch.schema_ref()).unwrap());
                writer.as_mut().unwrap()
            }
        };
        w.write(&batch).unwrap();
        stdout().flush().unwrap();
    }
    if let Some(mut w) = writer {
        w.finish().unwrap();
    }
}
