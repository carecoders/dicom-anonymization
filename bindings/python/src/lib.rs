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

/// Lightning-fast DICOM anonymization for Python, written in Rust.
///
/// The Anonymizer class provides methods to anonymize DICOM files by applying
/// various actions to specific DICOM tags such as removing, hashing, or replacing
/// patient identifiable information.
///
/// Args:
///     uid_root (str, optional): UID root to use for generating new UIDs. Defaults to "9999".
///     tag_actions (dict, optional): Dictionary mapping DICOM tags to anonymization actions.
///         Keys should be tag strings in format "GGGGEEEE" and values should be action
///         dictionaries with an "action" key. Available actions: "empty", "hash", "hashdate",
///         "hashuid", "keep", "none", "remove", "replace".
///
/// Returns:
///     Anonymizer: A new Anonymizer instance configured with the specified settings.
///
/// Example:
///     >>> from dcmanon import Anonymizer
///     >>> anonymizer = Anonymizer()
///     >>> anonymized_data = anonymizer.anonymize("input.dcm")
///
///     >>> # with custom configuration
///     >>> tag_actions = {
///     ...     "(0010,0010)": {"action": "replace", "value": "Anonymous"},
///     ...     "(0010,0020)": {"action": "hash", "length": 16}
///     ... }
///     >>> anonymizer = Anonymizer(uid_root="1.2.840.123", tag_actions=tag_actions)
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
                                value: replace_value,
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

    /// Anonymize a DICOM file.
    ///
    /// Processes a DICOM file by applying the configured anonymization actions to
    /// remove, modify, or hash patient identifiable information according to the
    /// anonymization rules specified during Anonymizer construction.
    ///
    /// Args:
    ///     fp (str or file-like): Input DICOM file. Can be either:
    ///         - A string path to a DICOM file on disk
    ///         - A file-like object (e.g., BytesIO, open file) containing DICOM data
    ///
    /// Returns:
    ///     bytes: The anonymized DICOM file as bytes, ready to be written to disk
    ///         or processed further.
    ///
    /// Raises:
    ///     AnonymizationError: If the DICOM file cannot be processed or anonymized.
    ///     IOError: If the input file cannot be read or output cannot be generated.
    ///
    /// Example:
    ///     >>> anonymizer = Anonymizer()
    ///     >>> # from file path
    ///     >>> anonymized_bytes = anonymizer.anonymize("patient_scan.dcm")
    ///     >>> with open("anonymized_scan.dcm", "wb") as f:
    ///     ...     f.write(anonymized_bytes)
    ///
    ///     >>> # from file-like object
    ///     >>> from io import BytesIO
    ///     >>> with open("input.dcm", "rb") as f:
    ///     ...     dicom_data = BytesIO(f.read())
    ///     >>> anonymized_bytes = anonymizer.anonymize(dicom_data)
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
