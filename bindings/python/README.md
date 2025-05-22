# dcmanon

Lightning-fast DICOM anonymization for Python, written in Rust.

`dcmanon` is a high-performance DICOM anonymization library that provides flexible,
configurable anonymization of DICOM files. Built in Rust for maximum performance,
it offers Python bindings for easy integration into medical imaging workflows.

## Features

- **Fast**: Written in Rust for optimal performance
- **Flexible**: Configurable anonymization actions per DICOM tag
- **Safe**: Type-safe implementation with comprehensive error handling
- **Standards-compliant**: Follows DICOM standard anonymization best practices by default
- **Multiple actions**: Remove, replace, hash, keep or empty DICOM tags
- **UID management**: Consistent UID generation with configurable UID roots

## Installation

```bash
pip install dcmanon
```

## Quick Start

```python
from dcmanon import Anonymizer

# Basic anonymization with default settings
anonymizer = Anonymizer()
anonymized_bytes = anonymizer.anonymize("/path/to/dicom.dcm")

with open("anonymized.dcm", "wb") as f:
    f.write(anonymized_bytes)
```

## Examples

### Basic Anonymization

```python
from dcmanon import Anonymizer

# Use default anonymization settings
anonymizer = Anonymizer()
anonymized_data = anonymizer.anonymize("input.dcm")

with open("output.dcm", "wb") as f:
    f.write(anonymized_data)
```

### Custom UID Root

```python
from dcmanon import Anonymizer

# Set a custom UID root for consistent UID generation
uid_root = "1.2.3.4.5"
anonymizer = Anonymizer(uid_root=uid_root)
anonymized_data = anonymizer.anonymize("input.dcm")
```

### Custom Tag Actions

```python
from dcmanon import Anonymizer

# Define custom actions for specific DICOM tags
tag_actions = {
    "00080050": {  # AccessionNumber
        "action": "hash",
        "length": 10,
    },
    "00100010": {  # PatientName
        "action": "replace",
        "value": "Anonymous Patient",
    },
    "00100020": {  # PatientID
        "action": "hash",
        "length": 8,
    },
    "00100030": {  # PatientBirthDate
        "action": "empty",
    },
    "00331010": {  # private tag
        "action": "keep",  # Keep this tag as is
    }
}

anonymizer = Anonymizer(uid_root="1.2.3.4.5", tag_actions=tag_actions)
anonymized_data = anonymizer.anonymize("input.dcm")
```

### Batch Processing

```python
from pathlib import Path

from dcmanon import Anonymizer

# Process multiple DICOM files
anonymizer = Anonymizer(uid_root="1.2.3.4.5")

input_dir = Path("dicom_files")
output_dir = Path("anonymized_files")
output_dir.mkdir(exist_ok=True)

for dcm_file in input_dir.glob("*.dcm"):
    try:
        anonymized_data = anonymizer.anonymize(str(dcm_file))
        output_path = output_dir / dcm_file.name

        with open(output_path, "wb") as f:
            f.write(anonymized_data)

        print(f"Anonymized: {dcm_file.name}")
    except Exception as e:
        print(f"Error processing {dcm_file.name}: {e}")
```

### Advanced Configuration

```python
from dcmanon import Anonymizer

# Complex anonymization configuration (as an example, as most of these actions are already in the
# default config)
tag_actions = {
    # Patient information
    "00100010": {"action": "replace", "value": "PATIENT^ANONYMOUS"},
    "00100020": {"action": "hash", "length": 12},
    "00100030": {"action": "hashdate"},  # Hash birth date
    "00100040": {"action": "keep"},      # Keep patient sex

    # Study information
    "0020000D": {"action": "hashuid"},   # Study Instance UID
    "00200010": {"action": "hash", "length": 8},  # Study ID
    "00081030": {"action": "empty"},     # Study Description

    # Series information
    "0020000E": {"action": "hashuid"},   # Series Instance UID
    "00080060": {"action": "keep"},      # Modality

    # Instance information
    "00080018": {"action": "hashuid"},   # SOP Instance UID
}

anonymizer = Anonymizer(
    uid_root="1.2.840.99999",
    tag_actions=tag_actions
)

anonymized_data = anonymizer.anonymize("complex_dicom.dcm")
```

## Available Actions

- **`hash`**: Hash the value (with optional length limit)
- **`hash_date`**: Hash date values while preserving format
- **`hash_uid`**: Hash UID values while maintaining UID format
- **`empty`**: Set the tag value to empty
- **`replace`**: Replace with a specified value
- **`remove`**: Completely remove the DICOM tag
- **`keep`**: Keep the original tag (to keep certain private tags, for example)
- **`none`**: Do nothing (to disable actions from the default config)

## Error Handling

```python
from dcmanon import Anonymizer, AnonymizationError

anonymizer = Anonymizer()

try:
    anonymized_data = anonymizer.anonymize("input.dcm")
    with open("output.dcm", "wb") as f:
        f.write(anonymized_data)
    print("Anonymization successful!")
except FileNotFoundError:
    print("Input file not found")
except AnonymizationError as e:
    print(f"DICOM anonymization failed: {e}")
except IOError as e:
    print(f"File I/O error: {e}")
except Exception as e:
    print(f"Unexpected error: {e}")
```
