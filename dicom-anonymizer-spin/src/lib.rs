use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use dicom_anonymization::{
    Anonymizer,
    config::{Config, builder::ConfigBuilder},
    processor::DefaultProcessor,
};
use serde::{Deserialize, Serialize};
use spin_sdk::{
    http::{IntoResponse, Params, Request, Response, Router},
    http_component,
};
use std::io::Cursor;

#[derive(Deserialize)]
struct CustomAnonymizationRequest {
    dicom_data: String,
    config: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct CustomAnonymizationResponse {
    anonymized_data: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

#[http_component]
fn handle_request(req: Request) -> Response {
    let mut router = Router::new();
    router.post("/anonymize", anonymize_default);
    router.post("/anonymize/custom", anonymize_custom);
    router.handle(req)
}

fn anonymize_default(req: Request, _params: Params) -> Result<impl IntoResponse> {
    let body_bytes = req.into_body();
    if body_bytes.is_empty() {
        return Ok(error_response(
            400,
            "invalid_request",
            "Request body is empty",
        ));
    }

    let dicom_data = body_bytes.to_vec();
    match perform_anonymization(&dicom_data, None) {
        Ok(anonymized_data) => Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/dicom")
            .body(anonymized_data)
            .build()),
        Err(e) => Ok(handle_anonymization_error(e)),
    }
}

fn anonymize_custom(req: Request, _params: Params) -> Result<impl IntoResponse> {
    let body_bytes = req.into_body();
    if body_bytes.is_empty() {
        return Ok(error_response(
            400,
            "invalid_request",
            "Request body is empty",
        ));
    }

    let request: CustomAnonymizationRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            return Ok(error_response(
                400,
                "invalid_json",
                &format!("Failed to parse request JSON: {}", e),
            ));
        }
    };

    let dicom_data = match BASE64.decode(&request.dicom_data) {
        Ok(data) => data,
        Err(e) => {
            return Ok(error_response(
                400,
                "invalid_base64",
                &format!("Failed to decode base64 DICOM data: {}", e),
            ));
        }
    };

    let config = if let Some(config_json) = request.config {
        match serde_json::from_value::<Config>(config_json) {
            Ok(config) => Some(config),
            Err(e) => {
                return Ok(error_response(
                    400,
                    "invalid_config",
                    &format!("Invalid configuration: {}", e),
                ));
            }
        }
    } else {
        None
    };

    match perform_anonymization(&dicom_data, config.as_ref()) {
        Ok(anonymized_data) => {
            let response = CustomAnonymizationResponse {
                anonymized_data: BASE64.encode(&anonymized_data),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&response)?)
                .build())
        }
        Err(e) => Ok(handle_anonymization_error(e)),
    }
}

fn perform_anonymization(
    dicom_data: &[u8],
    config: Option<&Config>,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut config_builder = ConfigBuilder::default();

    if let Some(cfg) = config {
        config_builder = config_builder.from_config(cfg);
    }

    let final_config = config_builder.build();
    let processor = DefaultProcessor::new(final_config);
    let anonymizer = Anonymizer::new(processor);

    let cursor = Cursor::new(dicom_data);
    let result = anonymizer
        .anonymize(cursor)
        .map_err(|e| anyhow::anyhow!("Anonymization failed: {}", e))?;

    let mut output = Vec::new();
    result
        .write(&mut output)
        .map_err(|e| anyhow::anyhow!("Failed to write DICOM: {}", e))?;

    Ok(output)
}

fn handle_anonymization_error(e: anyhow::Error) -> Response {
    let error_msg = e.to_string();
    if error_msg.contains("Read error") || error_msg.contains("not a DICOM") {
        error_response(400, "invalid_dicom", "Invalid DICOM data provided")
    } else {
        error_response(
            500,
            "processing_error",
            &format!("Failed to process DICOM: {}", e),
        )
    }
}

fn error_response(status: u16, error: &str, message: &str) -> Response {
    let error_resp = ErrorResponse {
        error: error.to_string(),
        message: message.to_string(),
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&error_resp).unwrap_or_default())
        .build()
}
