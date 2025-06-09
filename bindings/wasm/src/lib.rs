use dicom_anonymization::{
    Anonymizer,
    config::{Config, builder::ConfigBuilder},
    processor::DefaultProcessor,
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
    pub fn new(config_json: Option<String>) -> Result<DicomAnonymizer, JsValue> {
        let mut config_builder = ConfigBuilder::default();

        config_builder = if let Some(json) = config_json {
            let config: Config = serde_json::from_str(&json)
                .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
            config_builder.from_config(&config)
        } else {
            config_builder
        };

        let config = config_builder.build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);

        Ok(DicomAnonymizer { anonymizer })
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

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
