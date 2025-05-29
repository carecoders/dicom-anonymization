use dicom_anonymization::config::builder::ConfigBuilder;
use dicom_anonymization::processor::DefaultProcessor;
use dicom_anonymization::Anonymizer as RustAnonymizer;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_file::PyFileLikeObject;
use pythonize::depythonize;
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
///     config (dict, optional): Configuration dictionary. Should match the structure of config_default.json.
///         This config determines what to override from the default configuration. Available actions:
///         "empty", "hash", "hashdate", "hashuid", "keep", "none", "remove", "replace".
///
/// Returns:
///     Anonymizer: A new Anonymizer instance configured with the specified settings.
///
/// Example:
///     >>> from dcmanon import Anonymizer
///     >>>
///     >>> # using default configuration
///     >>> anonymizer = Anonymizer()
///     >>> anonymized_data = anonymizer.anonymize("input.dcm")
///
///     >>> # providing overrides for the default configuration
///     >>> config = {
///     ...     "uid_root": "1.2.840.123",
///     ...     "remove_private_tags": True,
///     ...     "remove_curves": False,
///     ...     "remove_overlays": True,
///     ...     "tag_actions": {
///     ...         "(0010,0010)": {"action": "empty"},
///     ...         "(0010,0020)": {"action": "remove"}
///     ...     }
///     ... }
///     >>> anonymizer = Anonymizer(config=config)
#[pyclass]
struct Anonymizer {
    inner: RustAnonymizer,
}

#[pymethods]
impl Anonymizer {
    /// Create a new Anonymizer instance
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut config_builder = ConfigBuilder::default();

        config_builder = if let Some(config_dict) = config {
            let config = depythonize(config_dict)
                .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid config: {}", e)))?;
            config_builder.from_config(&config)
        } else {
            config_builder
        };

        let config = config_builder.build();
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
