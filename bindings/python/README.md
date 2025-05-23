# dcmanon

Lightning-fast DICOM anonymization for Python, written in Rust.

This is a high-performance DICOM anonymization library that provides flexible,
configurable anonymization of DICOM files. Built in Rust for maximum performance,
it offers Python bindings for easy integration into various different workflows.

## Features

- **Fast**: written in Rust for optimal performance
- **Flexible**: configurable anonymization actions per DICOM tag
- **Safe**: type-safe implementation with comprehensive error handling
- **Standards-compliant**: follows DICOM standard anonymization best practices by default
- **Multiple actions**: remove, replace, hash, keep or empty DICOM tags
- **UID management**: consistent UID generation with configurable UID roots

## Installation

```bash
pip install dcmanon
```

## Quick start

```python
from dcmanon import Anonymizer

anonymizer = Anonymizer()
anonymized_bytes = anonymizer.anonymize("input.dcm")

with open("anonymized.dcm", "wb") as f:
    f.write(anonymized_bytes)
```

### Default settings

When the `Anonymizer` is initialized without any arguments, the default anonymization settings are used. To see those
default settings, you can either find them [here](https://github.com/carecoders/dicom-anonymization/blob/main/dicom-anonymization/config_default.json)
or you can generate them yourself like this:

```bash
cargo install dcmanon
dcmanon config create -o config_default.json
```

You can customize the default UID root and tag actions by providing your own when you
initialize the `Anonymizer`. See the following examples for how to do this.

## Examples

### Custom UID root

```python
from dcmanon import Anonymizer

uid_root = "1.2.3.4.5"
anonymizer = Anonymizer(uid_root=uid_root)
anonymized_data = anonymizer.anonymize("input.dcm")
```

### Custom tag actions

Custom tag actions override the same tag actions as defined in the [default settings](https://github.com/carecoders/dicom-anonymization/blob/main/dicom-anonymization/config_default.json).
The other default tag actions will still be applied as well.

Any tag action as defined in the default settings can be overridden. And you can also
provide actions for tags that are not defined in the default settings, like specific
private tags, for example.

#### Available actions

- **`hash`**: Hash the value (with optional length, minimum is 8)
- **`hashdate`**: Hash date values while preserving format
- **`hashuid`**: Hash UID values while maintaining UID format and using the UID root
- **`empty`**: Set the tag value to empty
- **`replace`**: Replace with a specified value
- **`remove`**: Completely remove the DICOM tag
- **`keep`**: Keep the original tag and value (to keep certain private tags, for example)
- **`none`**: Do nothing (to disable/override actions from the default config)

```python
from dcmanon import Anonymizer

tag_actions = {
    "00080050": {  # AccessionNumber
        "action": "hash",
        "length": 10,
    },
    "00100010": {  # PatientName
        "action": "replace",
        "value": "Anonymous^Patient",
    },
    "00100020": {  # PatientID
        "action": "hash",
        "length": 8,
    },
    "00100030": {  # PatientBirthDate
        "action": "empty",
    },
    "00331010": {  # private tag
        # all private tags are removed (by default), except this one
        "action": "keep",
    }
}

anonymizer = Anonymizer(uid_root="1.2.3.4.5", tag_actions=tag_actions)
anonymized_data = anonymizer.anonymize("input.dcm")
```

### More advanced configuration

```python
from dcmanon import Anonymizer

tag_actions = {
    # patient information
    "00100010": {"action": "replace", "value": "PATIENT^ANONYMOUS"},
    "00100020": {"action": "hash", "length": 12},
    "00100030": {"action": "hashdate"},  # Hash birth date
    "00100040": {"action": "keep"},      # Keep patient sex (`"none"` does the same)

    # study information
    "0020000D": {"action": "hashuid"},   # Study Instance UID
    "00200010": {"action": "hash", "length": 8},  # Study ID
    "00081030": {"action": "empty"},     # Study Description

    # series information
    "0020000E": {"action": "hashuid"},   # Series Instance UID
    "00080060": {"action": "keep"},      # Modality

    # instance information
    "00080018": {"action": "hashuid"},   # SOP Instance UID
}

anonymizer = Anonymizer(
    uid_root="1.2.840.99999",
    tag_actions=tag_actions
)

anonymized_data = anonymizer.anonymize("input.dcm")
```

## Error handling

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
