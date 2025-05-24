import {DICOMAnonymizer, initializeWasm} from './dicom-anonymizer.js';

let selectedFile = null;
let anonymizedData = null;

const uploadArea = document.getElementById('uploadArea');
const fileInput = document.getElementById('fileInput');
const processingSection = document.getElementById('processingSection');
const fileName = document.getElementById('fileName');
const fileSize = document.getElementById('fileSize');
const loadingIndicator = document.getElementById('loadingIndicator');
const resultSection = document.getElementById('resultSection');
const errorSection = document.getElementById('errorSection');
const errorMessage = document.getElementById('errorMessage');
const downloadBtn = document.getElementById('downloadBtn');
const resetBtn = document.getElementById('resetBtn');
const retryBtn = document.getElementById('retryBtn');
const versionInfo = document.getElementById('versionInfo');

async function init() {
    try {
        await initializeWasm();
        versionInfo.textContent = `Version ${DICOMAnonymizer.getVersion()}`;
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        showError('Failed to initialize the application. Please refresh the page.');
    }
}

uploadArea.addEventListener('click', () => fileInput.click());
fileInput.addEventListener('change', handleFileSelect);

uploadArea.addEventListener('dragover', (e) => {
    e.preventDefault();
    uploadArea.classList.add('drag-over');
});

uploadArea.addEventListener('dragleave', () => {
    uploadArea.classList.remove('drag-over');
});

uploadArea.addEventListener('drop', (e) => {
    e.preventDefault();
    uploadArea.classList.remove('drag-over');

    const files = e.dataTransfer.files;
    if (files.length > 0) {
        handleFile(files[0]);
    }
});

function handleFileSelect(e) {
    const files = e.target.files;
    if (files.length > 0) {
        handleFile(files[0]);
    }
}

async function handleFile(file) {
    selectedFile = file;
    fileName.textContent = file.name;
    fileSize.textContent = formatFileSize(file.size);

    document.querySelector('.upload-section').style.display = 'none';
    processingSection.style.display = 'block';
    resultSection.style.display = 'none';
    errorSection.style.display = 'none';
    loadingIndicator.style.display = 'block'; // show loading immediately

    // small delay to ensure UI updates
    await new Promise(resolve => setTimeout(resolve, 100));

    // automatically start anonymization
    await anonymizeFile();
}

function formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';

    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));

    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

async function anonymizeFile() {
    if (!selectedFile) return;

    errorSection.style.display = 'none';

    try {
        const arrayBuffer = await readFileAsArrayBuffer(selectedFile);

        const anonymizer = new DICOMAnonymizer();
        anonymizedData = anonymizer.anonymize(arrayBuffer);

        showLoading(false);
        resultSection.style.display = 'block';
    } catch (error) {
        console.error('Anonymization error:', error);
        showLoading(false);
        showError(error.message || 'Failed to anonymize the file.');
    }
}

function readFileAsArrayBuffer(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (e) => resolve(e.target.result);
        reader.onerror = reject;
        reader.readAsArrayBuffer(file);
    });
}

function showLoading(show) {
    loadingIndicator.style.display = show ? 'block' : 'none';
}

function showError(message) {
    errorMessage.textContent = message;
    errorSection.style.display = 'block';
    resultSection.style.display = 'none';
}

downloadBtn.addEventListener('click', downloadAnonymizedFile);

function downloadAnonymizedFile() {
    if (!anonymizedData || !selectedFile) return;

    const blob = new Blob([anonymizedData], {type: 'application/dicom'});
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `anonymized_${selectedFile.name}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);

    URL.revokeObjectURL(url);
}

resetBtn.addEventListener('click', reset);
retryBtn.addEventListener('click', reset);

function reset() {
    selectedFile = null;
    anonymizedData = null;
    fileInput.value = '';

    document.querySelector('.upload-section').style.display = 'block';
    processingSection.style.display = 'none';
}

init();
