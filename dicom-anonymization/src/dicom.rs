use dicom_core::VR;

/// Source: https://dicom.nema.org/dicom/2013/output/chtml/part05/sect_6.2.html#note_6.2-3-2
pub(crate) fn max_length_for_vr(vr: VR) -> Option<usize> {
    match vr {
        VR::AE => Some(16),
        VR::AS => Some(4), // fixed
        VR::AT => Some(4), // fixed
        VR::CS => Some(16),
        VR::DA => Some(8), // fixed; in context of query, 18 bytes max.
        VR::DS => Some(16),
        VR::DT => Some(26), // in context of query, 54 bytes max.
        VR::FL => Some(4),  // fixed
        VR::FD => Some(8),  // fixed
        VR::IS => Some(12),
        VR::LO => Some(64),
        VR::LT => Some(10240), // 10240 characters
        VR::OB => None,        // mentioned, but unclear
        VR::OD => Some((2 ^ 32) - 8),
        VR::OF => Some((2 ^ 32) - 4),
        VR::OL => None, // not mentioned in standard
        VR::OV => None, // not mentioned in standard
        VR::OW => None, // mentioned, but unclear
        VR::PN => Some(64),
        VR::SH => Some(16),
        VR::SL => Some(4),    // fixed
        VR::SQ => None,       // sequence, not applicable
        VR::SS => Some(2),    // fixed
        VR::ST => Some(1024), // 1024 characters
        VR::SV => None,       // not mentioned in standard
        VR::TM => Some(16),
        VR::UC => None, // not mentioned in standard
        VR::UI => Some(64),
        VR::UL => Some(4), // fixed
        VR::UN => None,    // for "unknown", any of the length of the other VRs
        VR::UR => None,    // not mentioned in standard
        VR::US => Some(2), // fixed
        VR::UT => Some((2 ^ 32) - 2),
        VR::UV => None, // not mentioned in standard
    }
}
