from io import BytesIO

import pydicom
import pytest
from dcmanon import Anonymizer


def test_uid_root():
    uid_root = "1.2.3.4.5.6.7.8"

    config = {"uid_root": uid_root}
    anonymizer = Anonymizer(config)

    file_path = "../../dicom-anonymization/tests/data/test.dcm"
    result_as_bytes = anonymizer.anonymize(file_path)
    ds = pydicom.dcmread(BytesIO(result_as_bytes))

    assert ds.StudyInstanceUID.startswith(uid_root)
    assert ds.SeriesInstanceUID.startswith(uid_root)
    assert ds.SOPInstanceUID.startswith(uid_root)


def test_action_replace():
    patient_name = "John Doe"

    config = {
        "tag_actions": {
            "00100010": {
                "comment": "PatientName",
                "action": "replace",
                "value": patient_name,
            }
        }
    }
    anonymizer = Anonymizer(config)

    file_path = "../../dicom-anonymization/tests/data/test.dcm"
    result_as_bytes = anonymizer.anonymize(file_path)
    ds = pydicom.dcmread(BytesIO(result_as_bytes))

    assert ds.PatientName == patient_name


def test_action_hash():
    hash_length = 16

    config = {
        "tag_actions": {
            "00080050": {
                "comment": "AccessionNumber",
                "action": "hash",
                "length": hash_length,
            }
        }
    }
    file_path = "../../dicom-anonymization/tests/data/test.dcm"
    original_ds = pydicom.dcmread(file_path)
    anonymizer = Anonymizer(config=config)

    result_as_bytes = anonymizer.anonymize(file_path)
    ds = pydicom.dcmread(BytesIO(result_as_bytes))

    assert len(ds.AccessionNumber) == hash_length
    assert not ds.AccessionNumber == original_ds.AccessionNumber


@pytest.mark.parametrize("hash_length", [1, 3, 5, 7])
def test_action_hash_length_too_small(hash_length):
    config = {
        "tag_actions": {
            "00080050": {
                "comment": "AccessionNumber",
                "action": "hash",
                "length": hash_length,
            }
        }
    }

    with pytest.raises(ValueError) as excinfo:
        Anonymizer(config)

    assert "Validation error for tag 00080050: length: lower than 8" in str(
        excinfo.value)


def test_all_config_keys():
    uid_root = "3333"
    hash_length = 16
    config = {
        "remove_private_tags": False,
        "uid_root": uid_root,
        "tag_actions": {
            "00080050": {
                "comment": "AccessionNumber",
                "action": "hash",
                "length": hash_length,
            }
        }
    }

    file_path = "../../dicom-anonymization/tests/data/test.dcm"
    original_ds = pydicom.dcmread(file_path)
    anonymizer = Anonymizer(config=config)

    result_as_bytes = anonymizer.anonymize(file_path)
    ds = pydicom.dcmread(BytesIO(result_as_bytes))

    assert ds.StudyInstanceUID.startswith(uid_root)
    assert ds.SeriesInstanceUID.startswith(uid_root)
    assert ds.SOPInstanceUID.startswith(uid_root)

    assert len(ds.AccessionNumber) == hash_length
    assert not ds.AccessionNumber == original_ds.AccessionNumber

    # check that private tags are not removed
    assert ds[0x00090010].value == "GEMS_IDEN_01"
