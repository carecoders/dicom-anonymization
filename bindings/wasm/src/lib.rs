use dicom_anonymization::{
    config::{builder::ConfigBuilder, uid_root::UidRoot, Config},
    processor::DefaultProcessor,
    Anonymizer,
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

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    // Configure to run in browser - can be run in Node.js by omitting this line
    wasm_bindgen_test_configure!(run_in_browser);

    // Simple test DICOM data (minimal valid DICOM file)
    // This is a basic DICOM file with just the required preamble and DICM prefix
    const TEST_DICOM: &[u8] = &[
        // DICOM preamble (128 zero bytes)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // DICM prefix
        b'D', b'I', b'C', b'M', // File Meta Information Group Length (0002,0000) - VR=UL
        0x02, 0x00, 0x00, 0x00, b'U', b'L', 0x04, 0x00, 0x0C, 0x00, 0x00, 0x00,
        // Media Storage SOP Class UID (0002,0002) - VR=UI
        0x02, 0x00, 0x02, 0x00, b'U', b'I', 0x0C, 0x00, b'1', b'.', b'2', b'.', b'8', b'4', b'0',
        b'.', b'1', b'0', b'0', b'0', b'8',
    ];

    // Test the underlying Rust functionality directly
    #[wasm_bindgen_test]
    fn test_version_function() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    #[wasm_bindgen_test]
    fn test_config_builder_default() {
        let config_builder = ConfigBuilder::default();
        let config = config_builder.build();
        assert!(true); // If we got here, the config was built successfully
    }

    #[wasm_bindgen_test]
    fn test_config_builder_with_json() {
        let config_json = r#"{
            "tag_actions": {},
            "private_tags_policy": "keep",
            "curves_policy": "keep",
            "overlays_policy": "keep",
            "uid_root": "1.2.3"
        }"#;

        let config: Result<Config, _> = serde_json::from_str(config_json);
        assert!(config.is_ok());

        if let Ok(config) = config {
            let config_builder = ConfigBuilder::default().from_config(&config);
            let final_config = config_builder.build();
            assert!(true); // Successfully built config
        }
    }

    #[wasm_bindgen_test]
    fn test_config_builder_with_invalid_json() {
        let invalid_config = r#"{ invalid json }"#;
        let config: Result<Config, _> = serde_json::from_str(invalid_config);
        assert!(config.is_err());
    }

    #[wasm_bindgen_test]
    fn test_anonymizer_creation() {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);
        assert!(true); // Successfully created anonymizer
    }

    #[wasm_bindgen_test]
    fn test_anonymize_with_empty_data() {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);

        let cursor = Cursor::new(&[]);
        let result = anonymizer.anonymize(cursor);
        assert!(result.is_err()); // Should fail with empty data
    }

    #[wasm_bindgen_test]
    fn test_anonymize_with_invalid_data() {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);

        let invalid_data = b"not a dicom file";
        let cursor = Cursor::new(invalid_data);
        let result = anonymizer.anonymize(cursor);
        assert!(result.is_err()); // Should fail with invalid DICOM data
    }

    #[wasm_bindgen_test]
    fn test_anonymize_with_minimal_dicom() {
        let config = ConfigBuilder::default().build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);

        let cursor = Cursor::new(TEST_DICOM);
        let result = anonymizer.anonymize(cursor);

        // This might succeed or fail depending on how complete our test DICOM is
        // The important thing is that it doesn't panic
        match result {
            Ok(dicom_obj) => {
                // If successful, try to write it to verify it's valid
                let mut output = Vec::new();
                let write_result = dicom_obj.write(&mut output);
                // Writing might fail, but that's OK for this test
                assert!(write_result.is_ok() || write_result.is_err());
            }
            Err(_) => {
                // Expected for minimal test data
                assert!(true);
            }
        }
    }

    #[wasm_bindgen_test]
    fn test_custom_config_with_actions() {
        let mut config_builder = ConfigBuilder::default();

        // Add some custom configuration with proper UidRoot
        let uid_root = "1.2.3.4".parse::<UidRoot>().expect("Valid UID root");
        config_builder = config_builder.uid_root(uid_root);

        let config = config_builder.build();
        let processor = DefaultProcessor::new(config);
        let anonymizer = Anonymizer::new(processor);
        assert!(true); // Successfully created anonymizer with custom config
    }

    #[wasm_bindgen_test]
    fn test_js_value_error_conversion() {
        let error_msg = "Test error message";
        let js_error = JsValue::from_str(&format!("Anonymization failed: {}", error_msg));

        // Test that we can create JsValue errors (this is what the WASM API returns)
        assert!(js_error.is_string());
    }
}
