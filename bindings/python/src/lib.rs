use dicom_anonymization::Anonymizer;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3_file::PyFileLikeObject;
use std::fs::File;
use std::io::Read;

/// Represents either a `FilePath` or a `FileLike` object
#[derive(Debug)]
enum FilePathOrFileLike {
    FilePath(String),
    FileLike(PyFileLikeObject),
}

impl<'py> FromPyObject<'py> for FilePathOrFileLike {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        // file path
        if let Ok(string) = ob.extract::<String>() {
            return Ok(FilePathOrFileLike::FilePath(string));
        }

        // file-like
        let f = PyFileLikeObject::py_with_requirements(ob.clone(), true, false, true, false)?;
        Ok(FilePathOrFileLike::FileLike(f))
    }
}

/// Anonymize a DICOM object and return the anonymized DICOM object as bytes.
#[pyfunction]
fn anonymize(fp: FilePathOrFileLike) -> PyResult<Vec<u8>> {
    let anonymizer = Anonymizer::default();

    let file: Box<dyn Read> = match fp {
        FilePathOrFileLike::FilePath(s) => Box::new(File::open(s)?),
        FilePathOrFileLike::FileLike(f) => Box::new(f),
    };

    let result = anonymizer
        .anonymize(file)
        .map_err(|e| PyErr::new::<PyException, _>(e.to_string()))?;

    let mut output = Vec::<u8>::new();
    result
        .write(&mut output)
        .map_err(|e| PyErr::new::<PyException, _>(e.to_string()))?;

    Ok(output)
}

/// A Python module implemented in Rust.
#[pymodule]
fn dcmanon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(anonymize, m)?)?;

    Ok(())
}
