use dicom_anonymization::actions::hash::HASH_LENGTH_MINIMUM;
use dicom_anonymization::actions::Action;
use dicom_anonymization::config::builder::ConfigBuilder;
use dicom_anonymization::config::uid_root::UidRoot;
use dicom_anonymization::processor::DefaultProcessor;
use dicom_anonymization::{Anonymizer as RustAnonymizer, Tag};
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
    #[pyo3(signature = (uid_root=None, tag_actions=None))]
    fn new(uid_root: Option<&str>, tag_actions: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut builder = ConfigBuilder::default();

        // Apply uid root if provided
        if let Some(uid_root) = uid_root {
            let uid_root =
                UidRoot::new(uid_root).map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))?;
            builder = builder.uid_root(uid_root);
        }

        // Apply tag actions if provided
        if let Some(tag_actions_dict) = tag_actions {
            for item in tag_actions_dict.iter() {
                let (tag, action) = item;

                // get the tag
                let tag_str: String = tag.extract()?;
                let tag: Tag = tag_str.parse().map_err(|_| {
                    PyErr::new::<PyValueError, _>(format!("Failed to parse tag {}", tag_str))
                })?;

                // get the action
                let action_dict: Bound<'_, PyDict> = action.extract()?;
                let action_str = action_dict.get_item("action")?;
                let action = if let Some(action_str) = action_str {
                    let action_str: &str = action_str.extract()?;
                    match action_str {
                        "empty" => Action::Empty,
                        "hashdate" => Action::HashDate,
                        "hashuid" => Action::HashUID,
                        "keep" => Action::Keep,
                        "none" => Action::None,
                        "remove" => Action::Remove,
                        "hash" => {
                            let mut hash_length: Option<usize> = None;

                            if let Some(length) = action_dict.get_item("length")? {
                                hash_length = length.extract().map_err(|_| {
                                    PyErr::new::<PyValueError, _>(format!(
                                        "Failed to parse hash length for tag {}",
                                        tag_str
                                    ))
                                })?;
                            };

                            if let Some(hash_length) = hash_length {
                                if hash_length < HASH_LENGTH_MINIMUM {
                                    return Err(PyErr::new::<PyValueError, _>(format!(
                                        "Hash length must be at least {} (tag {})",
                                        HASH_LENGTH_MINIMUM, tag_str
                                    )));
                                }
                            }

                            Action::Hash {
                                length: hash_length,
                            }
                        }
                        "replace" => {
                            let replace_value =
                                if let Some(value) = action_dict.get_item("value")? {
                                    value.extract::<String>()?
                                } else {
                                    return Err(PyErr::new::<PyValueError, _>(format!(
                                        "Failed to find replace value for tag {}",
                                        tag_str
                                    )));
                                };

                            Action::Replace {
                                value: replace_value.into(),
                            }
                        }
                        _ => {
                            return Err(PyErr::new::<PyValueError, _>(format!(
                                "Unsupported action '{}' for tag {}. Should be one of: hash, hashdate, hashuid, empty, remove, replace, keep, none.",
                                action_str, tag_str
                            )));
                        }
                    }
                } else {
                    return Err(PyErr::new::<PyValueError, _>(format!(
                        "Failed to find action key for tag {}",
                        tag_str
                    )));
                };

                // Apply tag action to builder
                builder = builder.tag_action(tag, action);
            }
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
