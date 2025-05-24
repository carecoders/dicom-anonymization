import init, {DicomAnonymizer, get_version} from './pkg/dicom_anonymization_wasm.js';

let isInitialized = false;

export async function initializeWasm() {
    if (isInitialized) return;

    await init('./pkg/dicom_anonymization_wasm_bg.wasm');
    isInitialized = true;
}

export class DICOMAnonymizer {
    constructor() {
        if (!isInitialized) {
            throw new Error('WASM module not initialized. Call initializeWasm() first.');
        }
        this.anonymizer = new DicomAnonymizer();
    }

    /**
     * Anonymize a DICOM file
     * @param {ArrayBuffer|Uint8Array} dicomData - The DICOM file data
     * @returns {Uint8Array} The anonymized DICOM file data
     */
    anonymize(dicomData) {
        let data;
        if (dicomData instanceof ArrayBuffer) {
            data = new Uint8Array(dicomData);
        } else if (dicomData instanceof Uint8Array) {
            data = dicomData;
        } else {
            throw new Error('Input must be ArrayBuffer or Uint8Array');
        }

        try {
            return this.anonymizer.anonymize(data);
        } catch (error) {
            throw new Error(`Anonymization failed: ${error.message || error}`);
        }
    }

    /**
     * Get the version of the WASM module
     * @returns {string} Version string
     */
    static getVersion() {
        if (!isInitialized) {
            throw new Error('WASM module not initialized. Call initializeWasm() first.');
        }
        return get_version();
    }
}
