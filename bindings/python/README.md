# dcmanon

Lightning-fast DICOM anonymization for Python, written in Rust.

## Installation

```bash
pip install dcmanon
```

## Usage

```Python
from dcmanon import Anonymizer

uid_root = "9999.12.3.4.5"

# override some default tag actions
tag_actions = {
    "00080050": {  # AccessionNumber
        "action": "hash",
        "length": 10,
    },
    "00100010": {  # PatientName
        "action": "replace",
        "value": "John Doe",
    }
}

file_path = "/path/to/dicom.dcm"  # change this

anonymizer = Anonymizer(uid_root=uid_root, tag_actions=tag_actions)
anonymized_dicom_as_bytes = anonymizer.anonymize(file_path)

output_file = open("anonymized.dcm", "wb")
output_file.write(anonymized_dicom_as_bytes)
```
