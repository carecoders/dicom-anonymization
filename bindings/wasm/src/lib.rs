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

#[cfg(test)]
mod tests {
    use super::*;
    use dicom_core::value::Value;
    use dicom_core::{DataElement, VR};
    use dicom_dictionary_std::tags;
    use dicom_object::{DefaultDicomObject, FileMetaTableBuilder, OpenFileOptions};
    use std::io::Cursor;

    const CANONICAL_ROOT: &str = "2.25.3501920042.100";

    /// A minimal explicit-VR little-endian DICOM file with the canonical golden-vector UIDs.
    fn make_test_file() -> Vec<u8> {
        let meta = FileMetaTableBuilder::new()
            .media_storage_sop_class_uid("1.2.3")
            .media_storage_sop_instance_uid("1.2.3.4.5.6.7")
            .transfer_syntax("1.2.840.10008.1.2.1")
            .build()
            .unwrap();
        let mut obj = DefaultDicomObject::new_empty_with_meta(meta);
        obj.put(DataElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            Value::from("1.2.3.4.5"),
        ));
        obj.put(DataElement::new(
            tags::SERIES_INSTANCE_UID,
            VR::UI,
            Value::from("1.2.3.4.5.6"),
        ));
        obj.put(DataElement::new(
            tags::SOP_INSTANCE_UID,
            VR::UI,
            Value::from("1.2.3.4.5.6.7"),
        ));
        let mut buf = Vec::new();
        obj.write_all(&mut buf).unwrap();
        buf
    }

    fn uid_str(file: &DefaultDicomObject, tag: dicom_anonymization::Tag) -> String {
        file.element(tag)
            .unwrap()
            .value()
            .to_str()
            .unwrap()
            .into_owned()
    }

    /// The config JSON shape the consuming web app passes: the canonical UID root
    /// plus the SHA-256 hash algorithm. The derived UIDs must equal the shared
    /// golden vectors.
    #[test]
    fn config_json_sha256_produces_golden_uids() {
        let config = format!(
            r#"{{"uid_root": "{}", "hash_algorithm": "sha256"}}"#,
            CANONICAL_ROOT
        );
        let anonymizer = DicomAnonymizer::new(Some(config)).expect("config JSON should parse");

        let out = anonymizer.anonymize(&make_test_file()).expect("anonymize should succeed");
        let file = OpenFileOptions::new()
            .from_reader(Cursor::new(&out))
            .unwrap();

        assert_eq!(
            uid_str(&file, tags::STUDY_INSTANCE_UID),
            "2.25.3501920042.100.83554103981997929853173016752638312087386577"
        );
        assert_eq!(
            uid_str(&file, tags::SERIES_INSTANCE_UID),
            "2.25.3501920042.100.11565063281463045692094511316009653451771902"
        );
        assert_eq!(
            uid_str(&file, tags::SOP_INSTANCE_UID),
            "2.25.3501920042.100.86134114536617455996661681892008515039411444"
        );
        // The file meta must follow the (hashed) dataset SOP instance UID.
        assert_eq!(
            file.meta().media_storage_sop_instance_uid.as_str(),
            "2.25.3501920042.100.86134114536617455996661681892008515039411444"
        );
    }

    /// Without `hash_algorithm` in the JSON the default (BLAKE3) derivation is
    /// unchanged: same root, but a different (non-SHA-256) digest.
    #[test]
    fn config_json_without_algorithm_keeps_blake3_default() {
        let config = format!(r#"{{"uid_root": "{}"}}"#, CANONICAL_ROOT);
        let anonymizer = DicomAnonymizer::new(Some(config)).expect("config JSON should parse");

        let out = anonymizer.anonymize(&make_test_file()).expect("anonymize should succeed");
        let file = OpenFileOptions::new()
            .from_reader(Cursor::new(&out))
            .unwrap();

        let study = uid_str(&file, tags::STUDY_INSTANCE_UID);
        assert!(study.starts_with(CANONICAL_ROOT));
        assert_ne!(
            study,
            "2.25.3501920042.100.83554103981997929853173016752638312087386577"
        ); // the SHA-256 golden vector must *not* appear for the blake3 default
    }
}
