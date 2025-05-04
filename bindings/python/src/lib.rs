use std::fs::File;

use dicom_anonymization::Anonymizer;
use pyo3::prelude::*;

/// Anonymize a DICOM file at the given path and return the anonymized DICOM file as a byte array.
#[pyfunction]
fn anonymize(file_path: &str) -> PyResult<Vec<u8>> {
    let anonymizer = Anonymizer::default();
    let file = File::open(file_path)?;
    let result = anonymizer.anonymize(file).unwrap();
    let mut output = Vec::<u8>::new();
    result.write(&mut output).unwrap();
    Ok(output)
}

/// A Python module implemented in Rust.
#[pymodule]
fn dcmanon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(anonymize, m)?)?;
    Ok(())
}
