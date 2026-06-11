use pyo3::prelude::*;
use pyo3_arrow::PyRecordBatch;
use std::ffi::CString;
use std::io::{stdin, stdout};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;

fn get_from_py(batch: RecordBatch, main_fn: &Bound<'_, PyAny>, kwargs: &Bound<'_, pyo3::types::PyDict>) -> PyResult<PyRecordBatch> {
    main_fn.call((PyRecordBatch::new(batch),), Some(kwargs))?.extract()
}

pub fn run_script(path: &str, flags: Vec<(String, String)>) -> anyhow::Result<()> {
    let file_path = String::from(path);
    let filename_cstr = CString::new(file_path.clone())?;
    let module_name = std::path::Path::new(&file_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    let module_cstr = CString::new(module_name)?;

    Python::attach(|py| {
        let sys = py.import("sys")?;
        sys.getattr("path")?.call_method1("append", (".",))?;

        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            let glob = py.import("glob")?;
            let pattern = format!("{}/lib/python*/site-packages", venv);
            let matches: Vec<String> = glob.call_method1("glob", (pattern,))?.extract()?;
            for path in matches {
                sys.getattr("path")?.call_method1("append", (path,))?;
            }
        }

        let code = std::fs::read_to_string(file_path)?;
        let code_cstr = CString::new(code)?;
        let module = PyModule::from_code(py, code_cstr.as_c_str(), &filename_cstr, &module_cstr)?;

        let main_fn = module.getattr("__ril_main__")?;

        let kwargs = pyo3::types::PyDict::new(py);
        for (k, v) in flags {
            kwargs.set_item(k, v)?;
        }

        let mut reader = StreamReader::try_new(stdin(), None)?;
        let mut writer: Option<StreamWriter<_>> = None;
        let mut batch_index: usize = 0;

        for batch in &mut reader {
            let batch = batch?;
            let result = get_from_py(batch, &main_fn, &kwargs)
                .map_err(|e| {
                    let traceback = e.traceback(py)
                        .and_then(|tb| tb.format().ok())
                        .unwrap_or_default();
                    anyhow::anyhow!("batch {batch_index}:\n{traceback}{e}")
                })?
                .into_inner();
            batch_index += 1;
            let w = match writer {
                Some(ref mut w) => w,
                None => {
                    writer = Some(StreamWriter::try_new(stdout(), result.schema_ref())?);
                    writer.as_mut().unwrap()
                }
            };
            w.write(&result)?;
        }
        if let Some(mut w) = writer {
            w.finish()?;
        }

        Ok(())
    })
}
