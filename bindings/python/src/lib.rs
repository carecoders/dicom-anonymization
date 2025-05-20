use dicom_anonymization::config::builder::ConfigBuilder;
use dicom_anonymization::config::uid_root::UidRoot;
use dicom_anonymization::processor::DefaultProcessor;
use dicom_anonymization::Anonymizer as RustAnonymizer;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_file::PyFileLikeObject;
use std::fs::File;
use std::io::Read;

// Create a proper Python exception that derives from Exception
create_exception!(
    dcmanon,
    AnonymizationError,
    PyException,
    "Exception raised during DICOM anonymization"
);

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

/// Anonymizer class
#[pyclass]
struct Anonymizer {
    inner: RustAnonymizer,
}

#[pymethods]
impl Anonymizer {
    /// Create a new Anonymizer instance
    #[new]
    #[pyo3(signature = (uid_root=None, config=None))] // TODO: change config to tag_actions
    fn new(uid_root: Option<&str>, config: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut builder = ConfigBuilder::default();

        // Apply individual parameters (they take precedence)
        if let Some(uid_root) = uid_root {
            let uid_root =
                UidRoot::new(uid_root).map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))?;
            builder = builder.uid_root(uid_root);
        }

        // Apply settings from the config dictionary
        if let Some(config_dict) = config {
            // Process uid_root if not already set via parameter
            if uid_root.is_none() && config_dict.contains("uid_root")? {
                let uid_root_str: String = config_dict.get_item("uid_root")?.unwrap().extract()?;
                let uid_root = UidRoot::new(&uid_root_str)
                    .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))?;
                builder = builder.uid_root(uid_root);
            }

            // Additional config options can be added here following the same pattern
            // Example:
            // if config_dict.contains("option_name")? {
            //     let value: ValueType = config_dict.get_item("option_name")?.extract()?;
            //     builder = builder.option_name(value);
            // }
        }

        let config = builder.build();
        let processor = DefaultProcessor::new(config);

        let anonymizer = RustAnonymizer::new(processor);

        Ok(Anonymizer { inner: anonymizer })
    }

    /// Anonymize a DICOM object and return the anonymized DICOM object as bytes.
    fn anonymize(&self, fp: FilePathOrFileLike) -> PyResult<Vec<u8>> {
        let file: Box<dyn Read> =
            match fp {
                FilePathOrFileLike::FilePath(s) => Box::new(File::open(s).map_err(|e| {
                    PyErr::new::<PyIOError, _>(format!("Failed to open file: {}", e))
                })?),
                FilePathOrFileLike::FileLike(f) => Box::new(f),
            };

        let result = self
            .inner
            .anonymize(file)
            .map_err(|e| PyErr::new::<AnonymizationError, _>(e.to_string()))?;

        let mut output = Vec::<u8>::new();
        result
            .write(&mut output)
            .map_err(|e| PyErr::new::<PyIOError, _>(e.to_string()))?;

        Ok(output)
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn dcmanon(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add the exception to the module
    m.add("AnonymizationError", py.get_type::<AnonymizationError>())?;

    // Add classes
    m.add_class::<Anonymizer>()?;

    Ok(())
}
