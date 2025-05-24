use dicom_anonymization::{
    config::builder::ConfigBuilder, processor::DefaultProcessor, Anonymizer,
};
use std::io::Cursor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct DicomAnonymizer {
    anonymizer: Anonymizer,
}

#[wasm_bindgen]
impl DicomAnonymizer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);

        DicomAnonymizer { anonymizer }
    }

    #[wasm_bindgen]
    pub fn anonymize(&self, dicom_data: &[u8]) -> Result<Vec<u8>, JsValue> {
        let cursor = Cursor::new(dicom_data);

        match self.anonymizer.anonymize(cursor) {
            Ok(result) => {
                let mut output = Vec::new();
                result
                    .write(&mut output)
                    .map_err(|e| JsValue::from_str(&format!("Failed to write DICOM: {}", e)))?;
                Ok(output)
            }
            Err(e) => Err(JsValue::from_str(&format!("Anonymization failed: {}", e))),
        }
    }
}

impl Default for DicomAnonymizer {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
