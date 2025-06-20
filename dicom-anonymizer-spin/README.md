# DICOM Anonymization Spin HTTP component

This is a [Spin](https://spinframework.dev) HTTP component that provides DICOM anonymization as a web service. It offers two endpoints for anonymizing DICOM files with optional custom configuration.

## API endpoints

### 1. Simple anonymization

**Endpoint:** `POST /anonymize`

Anonymize DICOM data using the default configuration.

**Request:**
- Body: Raw DICOM file bytes
- Content-Type: `application/dicom` or `application/octet-stream`

**Response:**
- Success (200): Anonymized DICOM file bytes
- Error (400): Invalid DICOM data
- Error (500): Processing error

**Example:**
```bash
curl -X POST http://localhost:3000/anonymize \
  --data-binary @patient.dcm \
  -o anonymized.dcm
```

### 2. Custom configuration

**Endpoint:** `POST /anonymize/custom`

Anonymize DICOM data with custom configuration options.

**Request:**
- Content-Type: `application/json`
- Body:
```json
{
  "dicom_data": "base64_encoded_dicom_bytes",
  "config": {
    // Optional configuration object
  }
}
```

**Response:**
- Success (200): Anonymized DICOM file bytes
- Error (400): Invalid request, DICOM data, or configuration
- Error (500): Processing error

**Example:**
```bash
# Encode DICOM file to base64 and anonymize with custom config
curl -X POST http://localhost:3000/anonymize/custom \
  -H "Content-Type: application/json" \
  -d '{
    "dicom_data": "'$(base64 < patient.dcm)'",
    "config": {
      "remove_private_tags": false,
      "remove_curves": true,
      "remove_overlays": true,
      "tag_actions": {
        "(0010,0010)": {"action": "replace", "value": "ANONYMOUS"},
        "(0010,0020)": {"action": "keep"}
      }
    }
  }' \
  -o custom_anonymized.dcm
```

## Configuration options

The `config` object in the custom endpoint supports all options from the DICOM anonymization library:

- `remove_private_tags`: Boolean (default: true)
- `remove_curves`: Boolean (default: true)
- `remove_overlays`: Boolean (default: true)
- `uid_root`: String - Custom UID root for generating new UIDs
- `tag_actions`: Object mapping DICOM tags to actions

### Available actions

- `{"action": "empty"}` - Empty the tag value
- `{"action": "remove"}` - Remove the tag entirely
- `{"action": "keep"}` - Keep the original value
- `{"action": "hash"}` - Hash the value
- `{"action": "hash_date"}` - Hash date values while preserving intervals using the hash of the PatientID tag
- `{"action": "hash_uid"}` - Hash UID values
- `{"action": "replace", "value": "..."}` - Replace with specified value

## Building and running

### Build the component:
```bash
cd dicom-anonymizer-spin
spin build
```

### Run locally:
```bash
spin up
```

The service will be available at `http://localhost:3000`

### Deploy to Fermyon Cloud:
```bash
spin deploy
```

## Error responses

All error responses follow this format:
```json
{
  "error": "error_type",
  "message": "Human-readable error description"
}
```

Error types:
- `invalid_request` - Empty or malformed request
- `invalid_json` - Cannot parse JSON request body
- `invalid_base64` - Cannot decode base64 DICOM data
- `invalid_dicom` - Not a valid DICOM file
- `invalid_config` - Invalid configuration object
- `processing_error` - Error during anonymization

## Development

This component is part of the dicom-anonymization workspace. To work on it:

1. Make changes to the source code in `src/lib.rs`
2. Build with `spin build` or `cargo build --target wasm32-wasip1 --release`
3. Test with `spin up` and the example curl commands above

The component uses the `dicom-anonymization` library from the workspace, so any changes to the core library will be reflected when rebuilding the Spin component.
