#[cfg(test)]
use dicom_object::{FileMetaTable, FileMetaTableBuilder};

#[cfg(test)]
pub(crate) fn make_file_meta() -> FileMetaTable {
    FileMetaTableBuilder::new()
        .media_storage_sop_class_uid("1.2.3")
        .media_storage_sop_instance_uid("2.3.4")
        .transfer_syntax("1.2.840.10008.1.2.1") // Explicit VR Little Endian
        .build()
        .unwrap()
}
