use pyo3::prelude::*;
use pyo3_arrow::PyRecordBatch;
use std::ffi::CString;
use std::io::{stdin, stdout};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use crate::error::{RilError, emit_and_wrap};

fn get_from_py(batch: RecordBatch, main_fn: &Bound<'_, PyAny>, kwargs: &Bound<'_, pyo3::types::PyDict>) -> PyResult<PyRecordBatch> {
    main_fn.call((PyRecordBatch::new(batch),), Some(kwargs))?.extract()
}

fn py_err_to_anyhow(e: PyErr, py: Python<'_>) -> anyhow::Error {
    let traceback = e.traceback(py)
        .and_then(|tb| tb.format().ok())
        .unwrap_or_default();
    let msg = format!("{traceback}{e}");
    emit_and_wrap(RilError::Python { traceback: msg })
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

        // Main exec loop block
        // later: exec from binary rather than just pyfn

        let mut reader = StreamReader::try_new(stdin(), None)?;
        let mut writer: Option<StreamWriter<_>> = None;
        let mut batch_index: usize = 0;

        for batch in &mut reader {
            let batch = batch?;
            let result = get_from_py(batch, &main_fn, &kwargs)
                .map_err(|e| {
                    let base = py_err_to_anyhow(e, py);
                    anyhow::anyhow!("batch {batch_index}: {base}")
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
