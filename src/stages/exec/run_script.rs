use crate::buffer::{Sender, Receiver, BatchMessage};

use pyo3::prelude::*;
use pyo3_arrow::PyRecordBatch;
use std::ffi::CString;
use arrow::record_batch::RecordBatch;


fn get_from_py(batch: RecordBatch, main_fn: &Bound<'_, PyAny>, kwargs: &Bound<'_, pyo3::types::PyDict>) -> PyResult<PyRecordBatch> {
    main_fn.call((PyRecordBatch::new(batch),), Some(kwargs))?.extract()
}

pub fn run_script(path: &str, flags: Vec<(String, String)>, rx: Receiver, tx: Sender) -> anyhow::Result<()> {
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
        
        loop {
            match rx.recv()? {
                BatchMessage::Data(batch ) => 
                    tx.send(BatchMessage::Data(get_from_py(batch, &main_fn, &kwargs)?.into_inner()))?,
                BatchMessage::Done => break,
            }
        }
    
        tx.send(BatchMessage::Done)?;

        Ok(())
    })
}



        // let schema = Arc::new(Schema::new(vec![ //this should be streaming in
        //     Field::new("values", DataType::Float64, false),
        // ]));
        // let values: ArrayRef = Arc::new(Float64Array::from(vec![23.7_f64, 21.1_f64]));
        // let batch = RecordBatch::try_new(schema, vec![values])?;

        // let result_obj = main_fn.call1((PyRecordBatch::new(batch),))?;
        // let result: PyRecordBatch = result_obj.extract()?;
        // result.
        // println!("Rust Says:");
        // println!("{:?}", result.as_ref());

        //until received "done" (or error):
        // let batch = (batch from the receiver)
        // let result_obj = python call (pyrecbatch(batch))
        // let result: PRB = result_obj.extract()?;
        // let arrowResult: RecordBatch = result.into_inner()
        // while arrow result is a thing send it to the next