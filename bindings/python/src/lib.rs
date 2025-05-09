use dicom_anonymization::config::{ConfigBuilder, UidRoot};
use dicom_anonymization::processor::DefaultProcessor;
use dicom_anonymization::Anonymizer as RustAnonymizer;
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

/// Python class that wraps the Rust Anonymizer
#[pyclass]
struct Anonymizer {
    inner: RustAnonymizer,
}

#[pymethods]
impl Anonymizer {
    /// Create a new Anonymizer with optional configuration
    #[new]
    fn new(uid_root: Option<String>, remove_private_tags: Option<bool>) -> Self {
        let mut builder = ConfigBuilder::default();

        if let Some(uid_root) = uid_root {
            // TODO: handle `Err`
            let uid_root = UidRoot::new(&uid_root).unwrap();
            builder = builder.uid_root(uid_root);
        }

        if let Some(remove_private) = remove_private_tags {
            if remove_private {
                builder = builder.remove_private_tags(true);
            }
        }

        let config = builder.build();
        let processor = DefaultProcessor::new(config);

        let anonymizer = RustAnonymizer::new(processor);

        Anonymizer { inner: anonymizer }
    }

    /// Create a new Anonymizer with custom configuration
    #[staticmethod]
    fn with_config(remove_private_tags: Option<bool>) -> Self {
        let mut builder = ConfigBuilder::default();

        if let Some(remove_private_tags) = remove_private_tags {
            if remove_private_tags {
                builder = builder.remove_private_tags(true);
            }
        }

        let config = builder.build();
        let processor = DefaultProcessor::new(config);

        let anonymizer = RustAnonymizer::new(processor);

        Anonymizer { inner: anonymizer }
    }

    /// Anonymize a DICOM object and return the anonymized DICOM object as bytes.
    fn anonymize(&self, fp: FilePathOrFileLike) -> PyResult<Vec<u8>> {
        let file: Box<dyn Read> = match fp {
            FilePathOrFileLike::FilePath(s) => Box::new(File::open(s)?),
            FilePathOrFileLike::FileLike(f) => Box::new(f),
        };

        let result = self
            .inner
            .anonymize(file)
            .map_err(|e| PyErr::new::<PyException, _>(e.to_string()))?;

        let mut output = Vec::<u8>::new();
        result
            .write(&mut output)
            .map_err(|e| PyErr::new::<PyException, _>(e.to_string()))?;

        Ok(output)
    }
}

// /// Anonymize a DICOM object and return the anonymized DICOM object as bytes.
// ///
// /// This is a convenience function that uses the default Anonymizer.
// #[pyfunction]
// fn anonymize(fp: FilePathOrFileLike) -> PyResult<Vec<u8>> {
//     let anonymizer = Anonymizer::new();
//     anonymizer.anonymize(fp)
// }

/// A Python module implemented in Rust.
#[pymodule]
fn dcmanon(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Anonymizer>()?;
    // m.add_function(wrap_pyfunction!(anonymize, m)?)?;

    Ok(())
}
