# DICOM Anonymization WASM

WebAssembly bindings for the DICOM anonymization library, enabling DICOM file anonymization directly in the browser.

## Features

- 🔒 Client-side anonymization - files never leave the browser
- 🚀 Fast WebAssembly performance
- 📦 Simple API for JavaScript/TypeScript
- 🎨 Ready-to-use web interface

## Building

Prerequisites:
- Rust and Cargo
- wasm-pack (`cargo install wasm-pack`)
- Node.js (for serving the demo)

Build the WASM module:
```bash
cd bindings/wasm
wasm-pack build --target web --out-dir www/pkg
```

## Running the Demo

1. Build the WASM module (see above)
2. Serve the web files:
   ```bash
   cd bindings/wasm
   python3 -m http.server --directory www 8080
   # or
   npm run serve
   ```
3. Open http://localhost:8080 in your browser

## Usage in Your Own Project

1. Build the WASM module
2. Copy the `pkg` directory to your project
3. Import and use:

```javascript
import init, { DicomAnonymizer } from './pkg/dicom_anonymization_wasm.js';

// Initialize WASM
await init();

// Create anonymizer
const anonymizer = new DicomAnonymizer();

// Anonymize DICOM data
const fileData = new Uint8Array(/* your DICOM file data */);
const anonymizedData = anonymizer.anonymize(fileData);

// Save or process anonymizedData
```

## API

### `DicomAnonymizer`

Main class for DICOM anonymization.

#### `new DicomAnonymizer()`
Creates a new anonymizer instance with default configuration.

#### `anonymize(data: Uint8Array): Uint8Array`
Anonymizes DICOM data and returns the anonymized result.

### `get_version(): string`
Returns the version of the WASM module.
