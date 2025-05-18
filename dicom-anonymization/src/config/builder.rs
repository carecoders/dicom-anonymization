use crate::actions::Action;
use crate::actions::Action::HashUID;
use crate::config::uid_root::{UidRoot, UID_ROOT_DEFAULT_VALUE};
use crate::config::{Config, DEIDENTIFIER};
use crate::hasher::HashFn;
use crate::tags;
use crate::Tag;

/// A builder for [`Config`] to configure DICOM de-identification settings.
///
/// The builder provides methods to customize various aspects of de-identification, including:
/// - Setting the UID root prefix for generating UIDs
/// - Configuring actions for specific DICOM tags
/// - Setting policies for private tags, curves, and overlays
///
/// # Example
///
/// ```
/// use dicom_anonymization::config::builder::ConfigBuilder;
/// use dicom_anonymization::actions::Action;
/// use dicom_anonymization::tags;
///
/// let config = ConfigBuilder::new()
///     .uid_root("1.2.840.123".parse().unwrap())
///     .tag_action(tags::PATIENT_NAME, Action::Empty)
///     .tag_action(tags::PATIENT_ID, Action::Hash{length: None})
///     .remove_private_tags(true)
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigBuilder(pub Config);

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder(Config::default())
    }

    /// Build a [`Config`] directly from the provided [`Config`].
    ///
    /// This method takes the [`ConfigBuilder`] and then applies the settings from the provided [`Config`].
    /// This is useful for creating a modified version of or overriding an existing [`Config`].
    ///
    /// # Arguments
    ///
    /// * `config` - The [`Config`] to use
    ///
    /// # Example
    ///
    /// ```
    /// use dicom_anonymization::config::Config;
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    /// use dicom_anonymization::actions::Action;
    /// use dicom_anonymization::tags;
    ///
    /// ```
    pub fn from_config(mut self, config: &Config) -> Self {
        // only explicitly set or override these if they are not `None` in the given `Config`
        if let Some(uid_root) = config.uid_root.clone() {
            self.0.uid_root = Some(uid_root);
        }
        if let Some(remove_private_tags) = config.remove_private_tags {
            self.0.remove_private_tags = Some(remove_private_tags);
        }
        if let Some(remove_curves) = config.remove_curves {
            self.0.remove_curves = Some(remove_curves);
        }
        if let Some(remove_overlays) = config.remove_overlays {
            self.0.remove_overlays = Some(remove_overlays);
        }

        // override existing tag actions
        for (tag, action) in &config.tag_actions {
            self.0.tag_actions.insert(*tag, action.clone());
        }
        self
    }

    /// Sets a custom hash function for use in hash operations.
    ///
    /// The hash function will be used for all operations requiring hashing like generating new UIDs and
    /// computing hash values for specific elements.
    ///
    /// # Example
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    /// use dicom_anonymization::hasher::blake3_hash_fn;
    ///
    /// let config = ConfigBuilder::new()
    ///    .hash_fn(blake3_hash_fn)
    ///    .build();
    /// ```
    pub fn hash_fn(mut self, hash_fn: HashFn) -> Self {
        self.0.hash_fn = hash_fn;
        self
    }

    /// Sets the UID root for the configuration.
    ///
    /// The [`UidRoot`] provides the prefix that will be used when creating new UIDs with [`Action::HashUID`].
    /// It must follow DICOM UID format rules: start with a digit 1-9 and contain only numbers and dots.
    /// It must also have no more than 32 characters.
    ///
    /// Setting it is optional. In that case, no specific UID prefix will be used when creating new UIDs.
    ///
    /// # Example
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    ///
    /// let config = ConfigBuilder::new()
    ///     .uid_root("1.2.840.123".parse().unwrap())
    ///     .build();
    /// ```
    pub fn uid_root(mut self, uid_root: UidRoot) -> Self {
        self.0.uid_root = Some(uid_root);
        self
    }

    /// Sets the action to take for a specific DICOM tag.
    ///
    /// The action determines how the tag value will be handled during de-identification.
    ///
    /// # Arguments
    ///
    /// * `tag` - The DICOM tag to apply the action to
    /// * `action` - The [`Action`] to take
    ///
    /// # Examples
    ///
    /// ```
    /// use dicom_anonymization::actions::Action;
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    /// use dicom_anonymization::tags;
    /// use dicom_anonymization::Tag;
    ///
    /// let mut config_builder = ConfigBuilder::new();
    ///
    /// // No specific action, leave the tag and its value unchanged
    /// config_builder = config_builder.tag_action(tags::MODALITY, Action::None);
    ///
    /// // Remove the tag completely
    /// config_builder = config_builder.tag_action(tags::SERIES_DATE, Action::Remove);
    ///
    /// // Replace the tag value with an empty value
    /// config_builder = config_builder.tag_action(tags::PATIENT_SEX, Action::Empty);
    ///
    /// // Hash the value with a specified length
    /// config_builder = config_builder.tag_action(tags::PATIENT_ID, Action::Hash { length: Some(10) });
    ///
    /// // Hash a UID
    /// config_builder = config_builder.tag_action(tags::STUDY_INSTANCE_UID, Action::HashUID);
    ///
    /// // Replace a date with another date using a hash of the PatientID value to determine the offset
    /// config_builder = config_builder.tag_action(tags::STUDY_DATE, Action::HashDate);
    ///
    /// // Replace the tag value with a specific value
    /// config_builder = config_builder.tag_action(tags::DEIDENTIFICATION_METHOD, Action::Replace { value: "MYAPP".into() });
    ///
    /// // Keep the specified tag even when the related group is to be removed
    /// config_builder = config_builder.remove_private_tags(true).tag_action(Tag(0x0033, 0x0010), Action::Keep);
    /// ```
    pub fn tag_action(mut self, tag: Tag, action: Action) -> Self {
        self.0.tag_actions.insert(tag, action);
        self
    }

    /// Controls whether private DICOM tags will be removed during de-identification.
    ///
    /// Private DICOM tags are those with odd group numbers. This function configures whether
    /// these tags should be removed or preserved.
    ///
    /// By default (i.e. if not explicitly set to `false`) all private tags will be removed. If enabled,
    /// individual private tags can still be kept by setting a specific tag [`Action`] for those
    /// (except [`Action::None`]).
    ///
    /// # Arguments
    ///
    /// * `remove` - If `true`, all private tags will be removed. If `false`, they will be kept.
    ///
    /// # Examples
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    ///
    /// // Remove private tags (default)
    /// let config = ConfigBuilder::new()
    ///     .remove_private_tags(true)
    ///     .build();
    ///
    /// // Keep private tags
    /// let config = ConfigBuilder::new()
    ///     .remove_private_tags(false)
    ///     .build();
    /// ```
    pub fn remove_private_tags(mut self, remove: bool) -> Self {
        self.0.remove_private_tags = Some(remove);
        self
    }

    /// Controls whether DICOM curve tags (from groups `0x5000-0x50FF`) will be removed during de-identification.
    ///
    /// By default (i.e. if not explicitly set to `false`) all curve tags will be removed. If enabled,
    /// individual curve tags can still be kept by setting a specific tag [`Action`] for those
    /// (except [`Action::None`]).
    ///
    /// # Arguments
    ///
    /// * `remove` - If `true`, all curve tags will be removed. If `false`, they will be kept.
    ///
    /// # Examples
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    ///
    /// // Remove curve tags (default)
    /// let config = ConfigBuilder::new()
    ///     .remove_curves(true)
    ///     .build();
    ///
    /// // Keep curve tags
    /// let config = ConfigBuilder::new()
    ///     .remove_curves(false)
    ///     .build();
    /// ```
    pub fn remove_curves(mut self, remove: bool) -> Self {
        self.0.remove_curves = Some(remove);
        self
    }

    /// Controls whether DICOM overlay tags (from groups `0x6000-0x60FF`) will be removed during de-identification.
    ///
    /// By default (i.e. if not explicitly set to `false`) all overlay tags will be removed. If enabled,
    /// individual overlay tags can still be kept by setting a specific tag [`Action`] for those
    /// (except [`Action::None`]).
    ///
    /// # Arguments
    ///
    /// * `remove` - If `true`, all overlay tags will be removed. If `false`, they will be kept.
    ///
    /// # Examples
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    ///
    /// // Remove overlay tags (default)
    /// let config = ConfigBuilder::new()
    ///     .remove_overlays(true)
    ///     .build();
    ///
    /// // Keep overlay tags
    /// let config = ConfigBuilder::new()
    ///     .remove_overlays(false)
    ///     .build();
    /// ```
    pub fn remove_overlays(mut self, remove: bool) -> Self {
        self.0.remove_overlays = Some(remove);
        self
    }

    /// Transforms the [`ConfigBuilder`] into a [`Config`] with all configured options.
    ///
    /// # Example
    ///
    /// ```
    /// use dicom_anonymization::config::builder::ConfigBuilder;
    /// use dicom_anonymization::actions::Action;
    /// use dicom_anonymization::tags;
    /// use dicom_anonymization::Tag;
    ///
    /// let config = ConfigBuilder::new()
    ///     .uid_root("1.2.840.123".parse().unwrap())
    ///     .remove_private_tags(true)
    ///     .tag_action(tags::SOP_INSTANCE_UID, Action::HashUID)
    ///     .tag_action(tags::PATIENT_NAME, Action::Empty)
    ///     .tag_action(Tag(0x0033, 0x0010), Action::Keep)
    ///     .build();
    /// ```
    pub fn build(self) -> Config {
        self.0
    }
}

impl Default for ConfigBuilder {
    #[allow(deprecated)]
    /// Creates a new `ConfigBuilder` with the default configuration.
    ///
    /// The default configuration includes a standard set of tag actions for DICOM de-identification,
    /// as well as default settings for removing private tags, curves, and overlays. Also, a default
    /// [`UidRoot`] value is used (i.e. `"9999"`).
    ///
    /// Returns a `ConfigBuilder` initialized with these default settings, which can be further customized
    /// if needed before building the final [`Config`].
    fn default() -> Self {
        Self::new()
            .uid_root(UidRoot::new(UID_ROOT_DEFAULT_VALUE).unwrap())
            .remove_private_tags(true)
            .remove_curves(true)
            .remove_overlays(true)
            .tag_action(tags::SPECIFIC_CHARACTER_SET, Action::None)
            .tag_action(tags::IMAGE_TYPE, Action::None)
            .tag_action(tags::INSTANCE_CREATION_DATE, Action::HashDate)
            .tag_action(tags::INSTANCE_CREATION_TIME, Action::None)
            .tag_action(tags::INSTANCE_CREATOR_UID, Action::HashUID)
            .tag_action(tags::INSTANCE_COERCION_DATE_TIME, Action::HashDate) // nic
            .tag_action(tags::SOP_CLASS_UID, Action::None)
            .tag_action(tags::ACQUISITION_UID, Action::HashUID) // nic
            .tag_action(tags::SOP_INSTANCE_UID, Action::HashUID)
            .tag_action(tags::STUDY_DATE, Action::HashDate)
            .tag_action(tags::SERIES_DATE, Action::Remove)
            .tag_action(tags::ACQUISITION_DATE, Action::Remove)
            .tag_action(tags::CONTENT_DATE, Action::HashDate)
            .tag_action(tags::OVERLAY_DATE, Action::Remove)
            .tag_action(tags::CURVE_DATE, Action::Remove)
            .tag_action(tags::ACQUISITION_DATE_TIME, Action::Remove)
            .tag_action(tags::STUDY_TIME, Action::Empty)
            .tag_action(tags::SERIES_TIME, Action::Remove)
            .tag_action(tags::ACQUISITION_TIME, Action::Remove)
            .tag_action(tags::CONTENT_TIME, Action::Empty)
            .tag_action(tags::OVERLAY_TIME, Action::Remove)
            .tag_action(tags::CURVE_TIME, Action::Remove)
            .tag_action(tags::ACCESSION_NUMBER, Action::Hash { length: Some(16) })
            .tag_action(tags::QUERY_RETRIEVE_LEVEL, Action::None)
            .tag_action(tags::RETRIEVE_AE_TITLE, Action::None)
            .tag_action(tags::STATION_AE_TITLE, Action::None) // nic
            .tag_action(tags::INSTANCE_AVAILABILITY, Action::None)
            .tag_action(tags::FAILED_SOP_INSTANCE_UID_LIST, Action::HashUID)
            .tag_action(tags::MODALITY, Action::None)
            .tag_action(tags::MODALITIES_IN_STUDY, Action::None)
            .tag_action(tags::ANATOMIC_REGIONS_IN_STUDY_CODE_SEQUENCE, Action::None) // nic
            .tag_action(tags::CONVERSION_TYPE, Action::None)
            .tag_action(tags::PRESENTATION_INTENT_TYPE, Action::None)
            .tag_action(tags::MANUFACTURER, Action::Empty)
            .tag_action(tags::INSTITUTION_NAME, Action::Remove)
            .tag_action(tags::INSTITUTION_ADDRESS, Action::Remove)
            .tag_action(tags::INSTITUTION_CODE_SEQUENCE, Action::None)
            .tag_action(tags::REFERRING_PHYSICIAN_NAME, Action::Empty)
            .tag_action(tags::REFERRING_PHYSICIAN_ADDRESS, Action::Remove)
            .tag_action(tags::REFERRING_PHYSICIAN_TELEPHONE_NUMBERS, Action::Remove)
            .tag_action(
                tags::REFERRING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::CONSULTING_PHYSICIAN_NAME, Action::Remove) // nic
            .tag_action(
                tags::CONSULTING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            ) // nic
            .tag_action(tags::CODE_VALUE, Action::None)
            .tag_action(tags::CODING_SCHEME_DESIGNATOR, Action::None)
            .tag_action(tags::CODING_SCHEME_VERSION, Action::None)
            .tag_action(tags::CODE_MEANING, Action::None)
            .tag_action(tags::MAPPING_RESOURCE, Action::None)
            .tag_action(tags::CONTEXT_GROUP_VERSION, Action::None)
            .tag_action(tags::CONTEXT_GROUP_LOCAL_VERSION, Action::None)
            .tag_action(tags::EXTENDED_CODE_MEANING, Action::None) // nic
            .tag_action(tags::CODING_SCHEME_URL_TYPE, Action::None) // nic
            .tag_action(tags::CONTEXT_GROUP_EXTENSION_FLAG, Action::None)
            .tag_action(tags::CODING_SCHEME_UID, Action::HashUID)
            .tag_action(tags::CONTEXT_GROUP_EXTENSION_CREATOR_UID, Action::HashUID)
            .tag_action(tags::CODING_SCHEME_URL, Action::None) // nic
            .tag_action(tags::CONTEXT_IDENTIFIER, Action::None)
            .tag_action(tags::CODING_SCHEME_REGISTRY, Action::None) // nic
            .tag_action(tags::CODING_SCHEME_EXTERNAL_ID, Action::None) // nic
            .tag_action(tags::CODING_SCHEME_NAME, Action::None) // nic
            .tag_action(tags::CODING_SCHEME_RESPONSIBLE_ORGANIZATION, Action::None) // nic
            .tag_action(tags::CONTEXT_UID, Action::HashUID) // nic
            .tag_action(tags::MAPPING_RESOURCE_UID, Action::HashUID) // nic
            .tag_action(tags::LONG_CODE_VALUE, Action::None) // nic
            .tag_action(tags::URN_CODE_VALUE, Action::None) // nic
            .tag_action(tags::EQUIVALENT_CODE_SEQUENCE, Action::None) // nic
            .tag_action(tags::MAPPING_RESOURCE_NAME, Action::None) // nic
            .tag_action(tags::TIMEZONE_OFFSET_FROM_UTC, Action::Remove)
            // checked nic's until here
            .tag_action(tags::STATION_NAME, Action::Remove)
            .tag_action(tags::STUDY_DESCRIPTION, Action::None)
            .tag_action(tags::PROCEDURE_CODE_SEQUENCE, Action::None)
            .tag_action(tags::SERIES_DESCRIPTION, Action::None)
            .tag_action(tags::INSTITUTIONAL_DEPARTMENT_NAME, Action::Remove)
            .tag_action(tags::PHYSICIANS_OF_RECORD, Action::Remove)
            .tag_action(
                tags::PHYSICIANS_OF_RECORD_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::PERFORMING_PHYSICIAN_NAME, Action::Remove)
            .tag_action(
                tags::PERFORMING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::NAME_OF_PHYSICIANS_READING_STUDY, Action::Remove)
            .tag_action(
                tags::PHYSICIANS_READING_STUDY_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::OPERATORS_NAME, Action::Remove)
            .tag_action(tags::OPERATOR_IDENTIFICATION_SEQUENCE, Action::Remove)
            .tag_action(tags::ADMITTING_DIAGNOSES_DESCRIPTION, Action::Remove)
            .tag_action(tags::ADMITTING_DIAGNOSES_CODE_SEQUENCE, Action::Remove)
            .tag_action(tags::MANUFACTURER_MODEL_NAME, Action::Remove)
            .tag_action(tags::REFERENCED_RESULTS_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_STUDY_SEQUENCE, Action::Remove)
            .tag_action(
                tags::REFERENCED_PERFORMED_PROCEDURE_STEP_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::REFERENCED_SERIES_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_PATIENT_SEQUENCE, Action::Remove)
            .tag_action(tags::REFERENCED_VISIT_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_OVERLAY_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_IMAGE_SEQUENCE, Action::Remove)
            .tag_action(tags::REFERENCED_CURVE_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_INSTANCE_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_SOP_CLASS_UID, Action::None)
            .tag_action(tags::REFERENCED_SOP_INSTANCE_UID, Action::HashUID)
            .tag_action(tags::SOP_CLASSES_SUPPORTED, Action::None)
            .tag_action(tags::REFERENCED_FRAME_NUMBER, Action::None)
            .tag_action(tags::TRANSACTION_UID, Action::HashUID)
            .tag_action(tags::FAILURE_REASON, Action::None)
            .tag_action(tags::FAILED_SOP_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_SOP_SEQUENCE, Action::None)
            .tag_action(tags::DERIVATION_DESCRIPTION, Action::Remove)
            .tag_action(tags::SOURCE_IMAGE_SEQUENCE, Action::Remove)
            .tag_action(tags::STAGE_NAME, Action::None)
            .tag_action(tags::STAGE_NUMBER, Action::None)
            .tag_action(tags::NUMBER_OF_STAGES, Action::None)
            .tag_action(tags::VIEW_NUMBER, Action::None)
            .tag_action(tags::NUMBER_OF_EVENT_TIMERS, Action::None)
            .tag_action(tags::NUMBER_OF_VIEWS_IN_STAGE, Action::None)
            .tag_action(tags::EVENT_ELAPSED_TIMES, Action::None)
            .tag_action(tags::EVENT_TIMER_NAMES, Action::None)
            .tag_action(tags::START_TRIM, Action::None)
            .tag_action(tags::STOP_TRIM, Action::None)
            .tag_action(tags::RECOMMENDED_DISPLAY_FRAME_RATE, Action::None)
            .tag_action(tags::ANATOMIC_REGION_SEQUENCE, Action::None)
            .tag_action(tags::ANATOMIC_REGION_MODIFIER_SEQUENCE, Action::None)
            .tag_action(tags::PRIMARY_ANATOMIC_STRUCTURE_SEQUENCE, Action::None)
            .tag_action(
                tags::ANATOMIC_STRUCTURE_SPACE_OR_REGION_SEQUENCE,
                Action::None,
            )
            .tag_action(
                tags::PRIMARY_ANATOMIC_STRUCTURE_MODIFIER_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::TRANSDUCER_POSITION_SEQUENCE, Action::None)
            .tag_action(tags::TRANSDUCER_POSITION_MODIFIER_SEQUENCE, Action::None)
            .tag_action(tags::TRANSDUCER_ORIENTATION_SEQUENCE, Action::None)
            .tag_action(tags::TRANSDUCER_ORIENTATION_MODIFIER_SEQUENCE, Action::None)
            .tag_action(tags::IRRADIATION_EVENT_UID, HashUID)
            .tag_action(tags::IDENTIFYING_COMMENTS, Action::Remove)
            .tag_action(tags::FRAME_TYPE, Action::None)
            .tag_action(tags::REFERENCED_IMAGE_EVIDENCE_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_RAW_DATA_SEQUENCE, Action::None)
            .tag_action(tags::CREATOR_VERSION_UID, HashUID)
            .tag_action(tags::DERIVATION_IMAGE_SEQUENCE, Action::None)
            .tag_action(tags::SOURCE_IMAGE_EVIDENCE_SEQUENCE, Action::None)
            .tag_action(tags::PIXEL_PRESENTATION, Action::None)
            .tag_action(tags::VOLUMETRIC_PROPERTIES, Action::None)
            .tag_action(tags::VOLUME_BASED_CALCULATION_TECHNIQUE, Action::None)
            .tag_action(tags::COMPLEX_IMAGE_COMPONENT, Action::None)
            .tag_action(tags::ACQUISITION_CONTRAST, Action::None)
            .tag_action(tags::DERIVATION_CODE_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_PRESENTATION_STATE_SEQUENCE, Action::None)
            .tag_action(tags::PATIENT_NAME, Action::Hash { length: Some(10) })
            .tag_action(tags::PATIENT_ID, Action::Hash { length: Some(10) })
            .tag_action(tags::ISSUER_OF_PATIENT_ID, Action::Remove)
            .tag_action(tags::PATIENT_BIRTH_DATE, Action::HashDate)
            .tag_action(tags::PATIENT_BIRTH_TIME, Action::Remove)
            .tag_action(tags::PATIENT_SEX, Action::Empty)
            .tag_action(tags::PATIENT_INSURANCE_PLAN_CODE_SEQUENCE, Action::Remove)
            .tag_action(tags::PATIENT_PRIMARY_LANGUAGE_CODE_SEQUENCE, Action::Remove)
            .tag_action(tags::OTHER_PATIENT_I_DS, Action::Remove)
            .tag_action(tags::OTHER_PATIENT_NAMES, Action::Remove)
            .tag_action(tags::OTHER_PATIENT_I_DS_SEQUENCE, Action::Remove)
            .tag_action(tags::PATIENT_BIRTH_NAME, Action::Remove)
            .tag_action(tags::PATIENT_AGE, Action::Remove)
            .tag_action(tags::PATIENT_SIZE, Action::Remove)
            .tag_action(tags::PATIENT_WEIGHT, Action::Remove)
            .tag_action(tags::PATIENT_ADDRESS, Action::Remove)
            .tag_action(tags::INSURANCE_PLAN_IDENTIFICATION, Action::Remove)
            .tag_action(tags::PATIENT_MOTHER_BIRTH_NAME, Action::Remove)
            .tag_action(tags::MILITARY_RANK, Action::Remove)
            .tag_action(tags::BRANCH_OF_SERVICE, Action::Remove)
            .tag_action(tags::MEDICAL_RECORD_LOCATOR, Action::Remove)
            .tag_action(tags::MEDICAL_ALERTS, Action::Remove)
            .tag_action(tags::ALLERGIES, Action::Remove)
            .tag_action(tags::COUNTRY_OF_RESIDENCE, Action::Remove)
            .tag_action(tags::REGION_OF_RESIDENCE, Action::Remove)
            .tag_action(tags::PATIENT_TELEPHONE_NUMBERS, Action::Remove)
            .tag_action(tags::PATIENT_TELECOM_INFORMATION, Action::Remove) // nic
            .tag_action(tags::ETHNIC_GROUP, Action::Remove)
            .tag_action(tags::OCCUPATION, Action::Remove)
            .tag_action(tags::SMOKING_STATUS, Action::Remove)
            .tag_action(tags::ADDITIONAL_PATIENT_HISTORY, Action::Remove)
            .tag_action(tags::PREGNANCY_STATUS, Action::Remove)
            .tag_action(tags::LAST_MENSTRUAL_DATE, Action::Remove)
            .tag_action(tags::PATIENT_RELIGIOUS_PREFERENCE, Action::Remove)
            .tag_action(tags::PATIENT_SEX_NEUTERED, Action::Remove)
            .tag_action(tags::RESPONSIBLE_PERSON, Action::Remove)
            .tag_action(tags::RESPONSIBLE_ORGANIZATION, Action::Remove)
            .tag_action(tags::PATIENT_COMMENTS, Action::Remove)
            .tag_action(tags::CLINICAL_TRIAL_SPONSOR_NAME, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_PROTOCOL_ID, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_PROTOCOL_NAME, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_SITE_ID, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_SITE_NAME, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_SUBJECT_ID, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_SUBJECT_READING_ID, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_TIME_POINT_ID, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_TIME_POINT_DESCRIPTION, Action::None)
            .tag_action(tags::CLINICAL_TRIAL_COORDINATING_CENTER_NAME, Action::None)
            // patient identity removal behaviour can be overridden by the user, therefore we can't know
            // for sure whether this happened or not
            .tag_action(tags::PATIENT_IDENTITY_REMOVED, Action::Remove)
            .tag_action(
                tags::DEIDENTIFICATION_METHOD,
                Action::Replace {
                    value: DEIDENTIFIER.into(),
                },
            )
            .tag_action(tags::DEIDENTIFICATION_METHOD_CODE_SEQUENCE, Action::Remove)
            .tag_action(tags::CONTRAST_BOLUS_AGENT, Action::Empty)
            .tag_action(tags::CONTRAST_BOLUS_AGENT_SEQUENCE, Action::None)
            .tag_action(
                tags::CONTRAST_BOLUS_ADMINISTRATION_ROUTE_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::BODY_PART_EXAMINED, Action::None)
            .tag_action(tags::SCANNING_SEQUENCE, Action::None)
            .tag_action(tags::SEQUENCE_VARIANT, Action::None)
            .tag_action(tags::SCAN_OPTIONS, Action::None)
            .tag_action(tags::MR_ACQUISITION_TYPE, Action::None)
            .tag_action(tags::SEQUENCE_NAME, Action::None)
            .tag_action(tags::ANGIO_FLAG, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_INFORMATION_SEQUENCE, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_STOP_TIME, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_DOSE, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_CODE_SEQUENCE, Action::None)
            .tag_action(tags::ADDITIONAL_DRUG_SEQUENCE, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_NAME, Action::None)
            .tag_action(tags::INTERVENTION_DRUG_START_TIME, Action::None)
            .tag_action(tags::INTERVENTION_SEQUENCE, Action::None)
            .tag_action(tags::THERAPY_TYPE, Action::None)
            .tag_action(tags::INTERVENTION_STATUS, Action::None)
            .tag_action(tags::THERAPY_DESCRIPTION, Action::None)
            .tag_action(tags::CINE_RATE, Action::None)
            .tag_action(tags::SLICE_THICKNESS, Action::None)
            .tag_action(tags::KVP, Action::None)
            .tag_action(tags::COUNTS_ACCUMULATED, Action::None)
            .tag_action(tags::ACQUISITION_TERMINATION_CONDITION, Action::None)
            .tag_action(tags::EFFECTIVE_DURATION, Action::None)
            .tag_action(tags::ACQUISITION_START_CONDITION, Action::None)
            .tag_action(tags::ACQUISITION_START_CONDITION_DATA, Action::None)
            .tag_action(tags::ACQUISITION_TERMINATION_CONDITION_DATA, Action::None)
            .tag_action(tags::REPETITION_TIME, Action::None)
            .tag_action(tags::ECHO_TIME, Action::None)
            .tag_action(tags::INVERSION_TIME, Action::None)
            .tag_action(tags::NUMBER_OF_AVERAGES, Action::None)
            .tag_action(tags::IMAGING_FREQUENCY, Action::None)
            .tag_action(tags::IMAGED_NUCLEUS, Action::None)
            .tag_action(tags::ECHO_NUMBERS, Action::None)
            .tag_action(tags::MAGNETIC_FIELD_STRENGTH, Action::None)
            .tag_action(tags::SPACING_BETWEEN_SLICES, Action::None)
            .tag_action(tags::NUMBER_OF_PHASE_ENCODING_STEPS, Action::None)
            .tag_action(tags::DATA_COLLECTION_DIAMETER, Action::None)
            .tag_action(tags::ECHO_TRAIN_LENGTH, Action::None)
            .tag_action(tags::PERCENT_SAMPLING, Action::None)
            .tag_action(tags::PERCENT_PHASE_FIELD_OF_VIEW, Action::None)
            .tag_action(tags::PIXEL_BANDWIDTH, Action::None)
            .tag_action(tags::DEVICE_SERIAL_NUMBER, Action::Remove)
            .tag_action(tags::DEVICE_UID, Action::HashUID)
            .tag_action(tags::PLATE_ID, Action::Remove)
            .tag_action(tags::GENERATOR_ID, Action::Remove)
            .tag_action(tags::CASSETTE_ID, Action::Remove)
            .tag_action(tags::GANTRY_ID, Action::Remove)
            .tag_action(tags::SECONDARY_CAPTURE_DEVICE_ID, Action::None)
            .tag_action(tags::HARDCOPY_CREATION_DEVICE_ID, Action::None)
            .tag_action(tags::DATE_OF_SECONDARY_CAPTURE, Action::HashDate)
            .tag_action(tags::TIME_OF_SECONDARY_CAPTURE, Action::None)
            .tag_action(tags::SECONDARY_CAPTURE_DEVICE_MANUFACTURER, Action::None)
            .tag_action(tags::HARDCOPY_DEVICE_MANUFACTURER, Action::None)
            .tag_action(
                tags::SECONDARY_CAPTURE_DEVICE_MANUFACTURER_MODEL_NAME,
                Action::None,
            )
            .tag_action(
                tags::SECONDARY_CAPTURE_DEVICE_SOFTWARE_VERSIONS,
                Action::None,
            )
            .tag_action(tags::HARDCOPY_DEVICE_SOFTWARE_VERSION, Action::None)
            .tag_action(tags::HARDCOPY_DEVICE_MANUFACTURER_MODEL_NAME, Action::None)
            .tag_action(tags::SOFTWARE_VERSIONS, Action::Remove)
            .tag_action(tags::VIDEO_IMAGE_FORMAT_ACQUIRED, Action::None)
            .tag_action(tags::DIGITAL_IMAGE_FORMAT_ACQUIRED, Action::None)
            .tag_action(tags::PROTOCOL_NAME, Action::Remove)
            .tag_action(tags::CONTRAST_BOLUS_ROUTE, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_VOLUME, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_START_TIME, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_STOP_TIME, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_TOTAL_DOSE, Action::None)
            .tag_action(tags::SYRINGE_COUNTS, Action::None)
            .tag_action(tags::CONTRAST_FLOW_RATE, Action::None)
            .tag_action(tags::CONTRAST_FLOW_DURATION, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_INGREDIENT, Action::None)
            .tag_action(tags::CONTRAST_BOLUS_INGREDIENT_CONCENTRATION, Action::None)
            .tag_action(tags::SPATIAL_RESOLUTION, Action::None)
            .tag_action(tags::TRIGGER_TIME, Action::None)
            .tag_action(tags::TRIGGER_SOURCE_OR_TYPE, Action::None)
            .tag_action(tags::NOMINAL_INTERVAL, Action::None)
            .tag_action(tags::FRAME_TIME, Action::None)
            .tag_action(tags::CARDIAC_FRAMING_TYPE, Action::None)
            .tag_action(tags::FRAME_TIME_VECTOR, Action::None)
            .tag_action(tags::FRAME_DELAY, Action::None)
            .tag_action(tags::IMAGE_TRIGGER_DELAY, Action::None)
            .tag_action(tags::MULTIPLEX_GROUP_TIME_OFFSET, Action::None)
            .tag_action(tags::TRIGGER_TIME_OFFSET, Action::None)
            .tag_action(tags::SYNCHRONIZATION_TRIGGER, Action::None)
            .tag_action(tags::SYNCHRONIZATION_CHANNEL, Action::None)
            .tag_action(tags::TRIGGER_SAMPLE_POSITION, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL_ROUTE, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL_VOLUME, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL_START_TIME, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL_STOP_TIME, Action::None)
            .tag_action(tags::RADIONUCLIDE_TOTAL_DOSE, Action::None)
            .tag_action(tags::RADIONUCLIDE_HALF_LIFE, Action::None)
            .tag_action(tags::RADIONUCLIDE_POSITRON_FRACTION, Action::None)
            .tag_action(tags::RADIOPHARMACEUTICAL_SPECIFIC_ACTIVITY, Action::None)
            .tag_action(tags::BEAT_REJECTION_FLAG, Action::None)
            .tag_action(tags::LOW_RR_VALUE, Action::None)
            .tag_action(tags::HIGH_RR_VALUE, Action::None)
            .tag_action(tags::INTERVALS_ACQUIRED, Action::None)
            .tag_action(tags::INTERVALS_REJECTED, Action::None)
            .tag_action(tags::PVC_REJECTION, Action::None)
            .tag_action(tags::SKIP_BEATS, Action::None)
            .tag_action(tags::HEART_RATE, Action::None)
            .tag_action(tags::CARDIAC_NUMBER_OF_IMAGES, Action::None)
            .tag_action(tags::TRIGGER_WINDOW, Action::None)
            .tag_action(tags::RECONSTRUCTION_DIAMETER, Action::None)
            .tag_action(tags::DISTANCE_SOURCE_TO_DETECTOR, Action::None)
            .tag_action(tags::DISTANCE_SOURCE_TO_PATIENT, Action::None)
            .tag_action(
                tags::ESTIMATED_RADIOGRAPHIC_MAGNIFICATION_FACTOR,
                Action::None,
            )
            .tag_action(tags::GANTRY_DETECTOR_TILT, Action::None)
            .tag_action(tags::GANTRY_DETECTOR_SLEW, Action::None)
            .tag_action(tags::TABLE_HEIGHT, Action::None)
            .tag_action(tags::TABLE_TRAVERSE, Action::None)
            .tag_action(tags::TABLE_MOTION, Action::None)
            .tag_action(tags::TABLE_VERTICAL_INCREMENT, Action::None)
            .tag_action(tags::TABLE_LATERAL_INCREMENT, Action::None)
            .tag_action(tags::TABLE_LONGITUDINAL_INCREMENT, Action::None)
            .tag_action(tags::TABLE_ANGLE, Action::None)
            .tag_action(tags::TABLE_TYPE, Action::None)
            .tag_action(tags::ROTATION_DIRECTION, Action::None)
            .tag_action(tags::ANGULAR_POSITION, Action::None)
            .tag_action(tags::RADIAL_POSITION, Action::None)
            .tag_action(tags::SCAN_ARC, Action::None)
            .tag_action(tags::ANGULAR_STEP, Action::None)
            .tag_action(tags::CENTER_OF_ROTATION_OFFSET, Action::None)
            .tag_action(tags::FIELD_OF_VIEW_SHAPE, Action::None)
            .tag_action(tags::FIELD_OF_VIEW_DIMENSIONS, Action::None)
            .tag_action(tags::EXPOSURE_TIME, Action::None)
            .tag_action(tags::X_RAY_TUBE_CURRENT, Action::None)
            .tag_action(tags::EXPOSURE, Action::None)
            .tag_action(tags::EXPOSURE_INU_AS, Action::None)
            .tag_action(tags::AVERAGE_PULSE_WIDTH, Action::None)
            .tag_action(tags::RADIATION_SETTING, Action::None)
            .tag_action(tags::RECTIFICATION_TYPE, Action::None)
            .tag_action(tags::RADIATION_MODE, Action::None)
            .tag_action(tags::IMAGE_AND_FLUOROSCOPY_AREA_DOSE_PRODUCT, Action::None)
            .tag_action(tags::FILTER_TYPE, Action::None)
            .tag_action(tags::TYPE_OF_FILTERS, Action::None)
            .tag_action(tags::INTENSIFIER_SIZE, Action::None)
            .tag_action(tags::IMAGER_PIXEL_SPACING, Action::None)
            .tag_action(tags::GRID, Action::None)
            .tag_action(tags::GENERATOR_POWER, Action::None)
            .tag_action(tags::COLLIMATOR_GRID_NAME, Action::None)
            .tag_action(tags::COLLIMATOR_TYPE, Action::None)
            .tag_action(tags::FOCAL_DISTANCE, Action::None)
            .tag_action(tags::X_FOCUS_CENTER, Action::None)
            .tag_action(tags::Y_FOCUS_CENTER, Action::None)
            .tag_action(tags::FOCAL_SPOTS, Action::None)
            .tag_action(tags::ANODE_TARGET_MATERIAL, Action::None)
            .tag_action(tags::BODY_PART_THICKNESS, Action::None)
            .tag_action(tags::COMPRESSION_FORCE, Action::None)
            .tag_action(tags::DATE_OF_LAST_CALIBRATION, Action::HashDate)
            .tag_action(tags::TIME_OF_LAST_CALIBRATION, Action::None)
            .tag_action(tags::CONVOLUTION_KERNEL, Action::None)
            .tag_action(tags::ACTUAL_FRAME_DURATION, Action::None)
            .tag_action(tags::COUNT_RATE, Action::None)
            .tag_action(tags::PREFERRED_PLAYBACK_SEQUENCING, Action::None)
            .tag_action(tags::RECEIVE_COIL_NAME, Action::None)
            .tag_action(tags::TRANSMIT_COIL_NAME, Action::None)
            .tag_action(tags::PLATE_TYPE, Action::None)
            .tag_action(tags::PHOSPHOR_TYPE, Action::None)
            .tag_action(tags::SCAN_VELOCITY, Action::None)
            .tag_action(tags::WHOLE_BODY_TECHNIQUE, Action::None)
            .tag_action(tags::SCAN_LENGTH, Action::None)
            .tag_action(tags::ACQUISITION_MATRIX, Action::None)
            .tag_action(tags::IN_PLANE_PHASE_ENCODING_DIRECTION, Action::None)
            .tag_action(tags::FLIP_ANGLE, Action::None)
            .tag_action(tags::VARIABLE_FLIP_ANGLE_FLAG, Action::None)
            .tag_action(tags::SAR, Action::None)
            .tag_action(tags::D_BDT, Action::None)
            .tag_action(
                tags::ACQUISITION_DEVICE_PROCESSING_DESCRIPTION,
                Action::Remove,
            )
            .tag_action(tags::ACQUISITION_DEVICE_PROCESSING_CODE, Action::None)
            .tag_action(tags::CASSETTE_ORIENTATION, Action::None)
            .tag_action(tags::CASSETTE_SIZE, Action::None)
            .tag_action(tags::EXPOSURES_ON_PLATE, Action::None)
            .tag_action(tags::RELATIVE_X_RAY_EXPOSURE, Action::None)
            .tag_action(tags::COLUMN_ANGULATION, Action::None)
            .tag_action(tags::TOMO_LAYER_HEIGHT, Action::None)
            .tag_action(tags::TOMO_ANGLE, Action::None)
            .tag_action(tags::TOMO_TIME, Action::None)
            .tag_action(tags::TOMO_TYPE, Action::None)
            .tag_action(tags::TOMO_CLASS, Action::None)
            .tag_action(tags::NUMBER_OF_TOMOSYNTHESIS_SOURCE_IMAGES, Action::None)
            .tag_action(tags::POSITIONER_MOTION, Action::None)
            .tag_action(tags::POSITIONER_TYPE, Action::None)
            .tag_action(tags::POSITIONER_PRIMARY_ANGLE, Action::None)
            .tag_action(tags::POSITIONER_SECONDARY_ANGLE, Action::None)
            .tag_action(tags::POSITIONER_PRIMARY_ANGLE_INCREMENT, Action::None)
            .tag_action(tags::POSITIONER_SECONDARY_ANGLE_INCREMENT, Action::None)
            .tag_action(tags::DETECTOR_PRIMARY_ANGLE, Action::None)
            .tag_action(tags::DETECTOR_SECONDARY_ANGLE, Action::None)
            .tag_action(tags::SHUTTER_SHAPE, Action::None)
            .tag_action(tags::SHUTTER_LEFT_VERTICAL_EDGE, Action::None)
            .tag_action(tags::SHUTTER_RIGHT_VERTICAL_EDGE, Action::None)
            .tag_action(tags::SHUTTER_UPPER_HORIZONTAL_EDGE, Action::None)
            .tag_action(tags::SHUTTER_LOWER_HORIZONTAL_EDGE, Action::None)
            .tag_action(tags::CENTER_OF_CIRCULAR_SHUTTER, Action::None)
            .tag_action(tags::RADIUS_OF_CIRCULAR_SHUTTER, Action::None)
            .tag_action(tags::VERTICES_OF_THE_POLYGONAL_SHUTTER, Action::None)
            .tag_action(tags::SHUTTER_PRESENTATION_VALUE, Action::None)
            .tag_action(tags::SHUTTER_OVERLAY_GROUP, Action::None)
            .tag_action(tags::COLLIMATOR_SHAPE, Action::None)
            .tag_action(tags::COLLIMATOR_LEFT_VERTICAL_EDGE, Action::None)
            .tag_action(tags::COLLIMATOR_RIGHT_VERTICAL_EDGE, Action::None)
            .tag_action(tags::COLLIMATOR_UPPER_HORIZONTAL_EDGE, Action::None)
            .tag_action(tags::COLLIMATOR_LOWER_HORIZONTAL_EDGE, Action::None)
            .tag_action(tags::CENTER_OF_CIRCULAR_COLLIMATOR, Action::None)
            .tag_action(tags::RADIUS_OF_CIRCULAR_COLLIMATOR, Action::None)
            .tag_action(tags::VERTICES_OF_THE_POLYGONAL_COLLIMATOR, Action::None)
            .tag_action(tags::ACQUISITION_TIME_SYNCHRONIZED, Action::None)
            .tag_action(tags::TIME_SOURCE, Action::None)
            .tag_action(tags::TIME_DISTRIBUTION_PROTOCOL, Action::None)
            .tag_action(tags::ACQUISITION_COMMENTS, Action::Remove)
            .tag_action(tags::OUTPUT_POWER, Action::None)
            .tag_action(tags::TRANSDUCER_DATA, Action::None)
            .tag_action(tags::FOCUS_DEPTH, Action::None)
            .tag_action(tags::PROCESSING_FUNCTION, Action::None)
            .tag_action(tags::POSTPROCESSING_FUNCTION, Action::None)
            .tag_action(tags::MECHANICAL_INDEX, Action::None)
            .tag_action(tags::BONE_THERMAL_INDEX, Action::None)
            .tag_action(tags::CRANIAL_THERMAL_INDEX, Action::None)
            .tag_action(tags::SOFT_TISSUE_THERMAL_INDEX, Action::None)
            .tag_action(tags::SOFT_TISSUE_FOCUS_THERMAL_INDEX, Action::None)
            .tag_action(tags::SOFT_TISSUE_SURFACE_THERMAL_INDEX, Action::None)
            .tag_action(tags::DEPTH_OF_SCAN_FIELD, Action::None)
            .tag_action(tags::PATIENT_POSITION, Action::None)
            .tag_action(tags::VIEW_POSITION, Action::None)
            .tag_action(tags::PROJECTION_EPONYMOUS_NAME_CODE_SEQUENCE, Action::None)
            .tag_action(tags::IMAGE_TRANSFORMATION_MATRIX, Action::None)
            .tag_action(tags::IMAGE_TRANSLATION_VECTOR, Action::None)
            .tag_action(tags::SENSITIVITY, Action::None)
            .tag_action(tags::SEQUENCE_OF_ULTRASOUND_REGIONS, Action::None)
            .tag_action(tags::REGION_SPATIAL_FORMAT, Action::None)
            .tag_action(tags::REGION_DATA_TYPE, Action::None)
            .tag_action(tags::REGION_FLAGS, Action::None)
            .tag_action(tags::REGION_LOCATION_MIN_X0, Action::None)
            .tag_action(tags::REGION_LOCATION_MIN_Y0, Action::None)
            .tag_action(tags::REGION_LOCATION_MAX_X1, Action::None)
            .tag_action(tags::REGION_LOCATION_MAX_Y1, Action::None)
            .tag_action(tags::REFERENCE_PIXEL_X0, Action::None)
            .tag_action(tags::REFERENCE_PIXEL_Y0, Action::None)
            .tag_action(tags::PHYSICAL_UNITS_X_DIRECTION, Action::None)
            .tag_action(tags::PHYSICAL_UNITS_Y_DIRECTION, Action::None)
            .tag_action(tags::REFERENCE_PIXEL_PHYSICAL_VALUE_X, Action::None)
            .tag_action(tags::REFERENCE_PIXEL_PHYSICAL_VALUE_Y, Action::None)
            .tag_action(tags::PHYSICAL_DELTA_X, Action::None)
            .tag_action(tags::PHYSICAL_DELTA_Y, Action::None)
            .tag_action(tags::TRANSDUCER_FREQUENCY, Action::None)
            .tag_action(tags::TRANSDUCER_TYPE, Action::None)
            .tag_action(tags::PULSE_REPETITION_FREQUENCY, Action::None)
            .tag_action(tags::DOPPLER_CORRECTION_ANGLE, Action::None)
            .tag_action(tags::STEERING_ANGLE, Action::None)
            .tag_action(tags::DOPPLER_SAMPLE_VOLUME_X_POSITION, Action::None)
            .tag_action(tags::DOPPLER_SAMPLE_VOLUME_Y_POSITION, Action::None)
            .tag_action(tags::TM_LINE_POSITION_X0, Action::None)
            .tag_action(tags::TM_LINE_POSITION_Y0, Action::None)
            .tag_action(tags::TM_LINE_POSITION_X1, Action::None)
            .tag_action(tags::TM_LINE_POSITION_Y1, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_ORGANIZATION, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_MASK, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_RANGE_START, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_RANGE_STOP, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_PHYSICAL_UNITS, Action::None)
            .tag_action(tags::PIXEL_COMPONENT_DATA_TYPE, Action::None)
            .tag_action(tags::NUMBER_OF_TABLE_BREAK_POINTS, Action::None)
            .tag_action(tags::TABLE_OF_X_BREAK_POINTS, Action::None)
            .tag_action(tags::TABLE_OF_Y_BREAK_POINTS, Action::None)
            .tag_action(tags::NUMBER_OF_TABLE_ENTRIES, Action::None)
            .tag_action(tags::TABLE_OF_PIXEL_VALUES, Action::None)
            .tag_action(tags::TABLE_OF_PARAMETER_VALUES, Action::None)
            .tag_action(tags::DETECTOR_CONDITIONS_NOMINAL_FLAG, Action::None)
            .tag_action(tags::DETECTOR_TEMPERATURE, Action::None)
            .tag_action(tags::DETECTOR_TYPE, Action::None)
            .tag_action(tags::DETECTOR_CONFIGURATION, Action::None)
            .tag_action(tags::DETECTOR_DESCRIPTION, Action::None)
            .tag_action(tags::DETECTOR_MODE, Action::None)
            .tag_action(tags::DETECTOR_ID, Action::Remove)
            .tag_action(tags::DATE_OF_LAST_DETECTOR_CALIBRATION, Action::HashDate)
            .tag_action(tags::TIME_OF_LAST_DETECTOR_CALIBRATION, Action::None)
            .tag_action(
                tags::EXPOSURES_ON_DETECTOR_SINCE_LAST_CALIBRATION,
                Action::None,
            )
            .tag_action(tags::EXPOSURES_ON_DETECTOR_SINCE_MANUFACTURED, Action::None)
            .tag_action(tags::DETECTOR_TIME_SINCE_LAST_EXPOSURE, Action::None)
            .tag_action(tags::DETECTOR_ACTIVE_TIME, Action::None)
            .tag_action(tags::DETECTOR_ACTIVATION_OFFSET_FROM_EXPOSURE, Action::None)
            .tag_action(tags::DETECTOR_BINNING, Action::None)
            .tag_action(tags::DETECTOR_ELEMENT_PHYSICAL_SIZE, Action::None)
            .tag_action(tags::DETECTOR_ELEMENT_SPACING, Action::None)
            .tag_action(tags::DETECTOR_ACTIVE_SHAPE, Action::None)
            .tag_action(tags::DETECTOR_ACTIVE_DIMENSIONS, Action::None)
            .tag_action(tags::DETECTOR_ACTIVE_ORIGIN, Action::None)
            .tag_action(tags::FIELD_OF_VIEW_ORIGIN, Action::None)
            .tag_action(tags::FIELD_OF_VIEW_ROTATION, Action::None)
            .tag_action(tags::FIELD_OF_VIEW_HORIZONTAL_FLIP, Action::None)
            .tag_action(tags::GRID_ABSORBING_MATERIAL, Action::None)
            .tag_action(tags::GRID_SPACING_MATERIAL, Action::None)
            .tag_action(tags::GRID_THICKNESS, Action::None)
            .tag_action(tags::GRID_PITCH, Action::None)
            .tag_action(tags::GRID_ASPECT_RATIO, Action::None)
            .tag_action(tags::GRID_PERIOD, Action::None)
            .tag_action(tags::GRID_FOCAL_DISTANCE, Action::None)
            .tag_action(tags::FILTER_MATERIAL, Action::None)
            .tag_action(tags::FILTER_THICKNESS_MINIMUM, Action::None)
            .tag_action(tags::FILTER_THICKNESS_MAXIMUM, Action::None)
            .tag_action(tags::EXPOSURE_CONTROL_MODE, Action::None)
            .tag_action(tags::EXPOSURE_CONTROL_MODE_DESCRIPTION, Action::None)
            .tag_action(tags::EXPOSURE_STATUS, Action::None)
            .tag_action(tags::PHOTOTIMER_SETTING, Action::None)
            .tag_action(tags::EXPOSURE_TIME_INU_S, Action::None)
            .tag_action(tags::X_RAY_TUBE_CURRENT_INU_A, Action::None)
            .tag_action(tags::CONTENT_QUALIFICATION, Action::None)
            .tag_action(tags::PULSE_SEQUENCE_NAME, Action::None)
            .tag_action(tags::MR_IMAGING_MODIFIER_SEQUENCE, Action::None)
            .tag_action(tags::ECHO_PULSE_SEQUENCE, Action::None)
            .tag_action(tags::INVERSION_RECOVERY, Action::None)
            .tag_action(tags::FLOW_COMPENSATION, Action::None)
            .tag_action(tags::MULTIPLE_SPIN_ECHO, Action::None)
            .tag_action(tags::MULTI_PLANAR_EXCITATION, Action::None)
            .tag_action(tags::PHASE_CONTRAST, Action::None)
            .tag_action(tags::TIME_OF_FLIGHT_CONTRAST, Action::None)
            .tag_action(tags::SPOILING, Action::None)
            .tag_action(tags::STEADY_STATE_PULSE_SEQUENCE, Action::None)
            .tag_action(tags::ECHO_PLANAR_PULSE_SEQUENCE, Action::None)
            .tag_action(tags::TAG_ANGLE_FIRST_AXIS, Action::None)
            .tag_action(tags::MAGNETIZATION_TRANSFER, Action::None)
            .tag_action(tags::T2_PREPARATION, Action::None)
            .tag_action(tags::BLOOD_SIGNAL_NULLING, Action::None)
            .tag_action(tags::SATURATION_RECOVERY, Action::None)
            .tag_action(tags::SPECTRALLY_SELECTED_SUPPRESSION, Action::None)
            .tag_action(tags::SPECTRALLY_SELECTED_EXCITATION, Action::None)
            .tag_action(tags::SPATIAL_PRESATURATION, Action::None)
            .tag_action(tags::TAGGING, Action::None)
            .tag_action(tags::OVERSAMPLING_PHASE, Action::None)
            .tag_action(tags::TAG_SPACING_FIRST_DIMENSION, Action::None)
            .tag_action(tags::GEOMETRY_OF_K_SPACE_TRAVERSAL, Action::None)
            .tag_action(tags::SEGMENTED_K_SPACE_TRAVERSAL, Action::None)
            .tag_action(tags::RECTILINEAR_PHASE_ENCODE_REORDERING, Action::None)
            .tag_action(tags::TAG_THICKNESS, Action::None)
            .tag_action(tags::PARTIAL_FOURIER_DIRECTION, Action::None)
            .tag_action(tags::CARDIAC_SYNCHRONIZATION_TECHNIQUE, Action::None)
            .tag_action(tags::RECEIVE_COIL_MANUFACTURER_NAME, Action::None)
            .tag_action(tags::MR_RECEIVE_COIL_SEQUENCE, Action::None)
            .tag_action(tags::RECEIVE_COIL_TYPE, Action::None)
            .tag_action(tags::QUADRATURE_RECEIVE_COIL, Action::None)
            .tag_action(tags::MULTI_COIL_DEFINITION_SEQUENCE, Action::None)
            .tag_action(tags::MULTI_COIL_CONFIGURATION, Action::None)
            .tag_action(tags::MULTI_COIL_ELEMENT_NAME, Action::None)
            .tag_action(tags::MULTI_COIL_ELEMENT_USED, Action::None)
            .tag_action(tags::MR_TRANSMIT_COIL_SEQUENCE, Action::None)
            .tag_action(tags::TRANSMIT_COIL_MANUFACTURER_NAME, Action::None)
            .tag_action(tags::TRANSMIT_COIL_TYPE, Action::None)
            .tag_action(tags::SPECTRAL_WIDTH, Action::None)
            .tag_action(tags::CHEMICAL_SHIFT_REFERENCE, Action::None)
            .tag_action(tags::VOLUME_LOCALIZATION_TECHNIQUE, Action::None)
            .tag_action(tags::MR_ACQUISITION_FREQUENCY_ENCODING_STEPS, Action::None)
            .tag_action(tags::DECOUPLING, Action::None)
            .tag_action(tags::DECOUPLED_NUCLEUS, Action::None)
            .tag_action(tags::DECOUPLING_FREQUENCY, Action::None)
            .tag_action(tags::DECOUPLING_METHOD, Action::None)
            .tag_action(tags::DECOUPLING_CHEMICAL_SHIFT_REFERENCE, Action::None)
            .tag_action(tags::K_SPACE_FILTERING, Action::None)
            .tag_action(tags::TIME_DOMAIN_FILTERING, Action::None)
            .tag_action(tags::NUMBER_OF_ZERO_FILLS, Action::None)
            .tag_action(tags::BASELINE_CORRECTION, Action::None)
            .tag_action(tags::CARDIAC_RR_INTERVAL_SPECIFIED, Action::None)
            .tag_action(tags::ACQUISITION_DURATION, Action::None)
            .tag_action(tags::FRAME_ACQUISITION_DATE_TIME, Action::HashDate)
            .tag_action(tags::DIFFUSION_DIRECTIONALITY, Action::None)
            .tag_action(tags::DIFFUSION_GRADIENT_DIRECTION_SEQUENCE, Action::None)
            .tag_action(tags::PARALLEL_ACQUISITION, Action::None)
            .tag_action(tags::PARALLEL_ACQUISITION_TECHNIQUE, Action::None)
            .tag_action(tags::INVERSION_TIMES, Action::None)
            .tag_action(tags::METABOLITE_MAP_DESCRIPTION, Action::None)
            .tag_action(tags::PARTIAL_FOURIER, Action::None)
            .tag_action(tags::EFFECTIVE_ECHO_TIME, Action::None)
            .tag_action(tags::CHEMICAL_SHIFT_SEQUENCE, Action::None)
            .tag_action(tags::CARDIAC_SIGNAL_SOURCE, Action::None)
            .tag_action(tags::DIFFUSION_B_VALUE, Action::None)
            .tag_action(tags::DIFFUSION_GRADIENT_ORIENTATION, Action::None)
            .tag_action(tags::VELOCITY_ENCODING_DIRECTION, Action::None)
            .tag_action(tags::VELOCITY_ENCODING_MINIMUM_VALUE, Action::None)
            .tag_action(tags::NUMBER_OF_K_SPACE_TRAJECTORIES, Action::None)
            .tag_action(tags::COVERAGE_OF_K_SPACE, Action::None)
            .tag_action(tags::SPECTROSCOPY_ACQUISITION_PHASE_ROWS, Action::None)
            .tag_action(tags::PARALLEL_REDUCTION_FACTOR_IN_PLANE, Action::None)
            .tag_action(tags::TRANSMITTER_FREQUENCY, Action::None)
            .tag_action(tags::RESONANT_NUCLEUS, Action::None)
            .tag_action(tags::FREQUENCY_CORRECTION, Action::None)
            .tag_action(tags::MR_SPECTROSCOPY_FOV_GEOMETRY_SEQUENCE, Action::None)
            .tag_action(tags::SLAB_THICKNESS, Action::None)
            .tag_action(tags::SLAB_ORIENTATION, Action::None)
            .tag_action(tags::MID_SLAB_POSITION, Action::None)
            .tag_action(tags::MR_SPATIAL_SATURATION_SEQUENCE, Action::None)
            .tag_action(
                tags::MR_TIMING_AND_RELATED_PARAMETERS_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::MR_ECHO_SEQUENCE, Action::None)
            .tag_action(tags::MR_MODIFIER_SEQUENCE, Action::None)
            .tag_action(tags::MR_DIFFUSION_SEQUENCE, Action::None)
            .tag_action(tags::CARDIAC_SYNCHRONIZATION_SEQUENCE, Action::None)
            .tag_action(tags::MR_AVERAGES_SEQUENCE, Action::None)
            .tag_action(tags::MRFOV_GEOMETRY_SEQUENCE, Action::None)
            .tag_action(tags::VOLUME_LOCALIZATION_SEQUENCE, Action::None)
            .tag_action(tags::SPECTROSCOPY_ACQUISITION_DATA_COLUMNS, Action::None)
            .tag_action(tags::DIFFUSION_ANISOTROPY_TYPE, Action::None)
            .tag_action(tags::FRAME_REFERENCE_DATE_TIME, Action::HashDate)
            .tag_action(tags::MR_METABOLITE_MAP_SEQUENCE, Action::None)
            .tag_action(tags::PARALLEL_REDUCTION_FACTOR_OUT_OF_PLANE, Action::None)
            .tag_action(
                tags::SPECTROSCOPY_ACQUISITION_OUT_OF_PLANE_PHASE_STEPS,
                Action::None,
            )
            .tag_action(tags::BULK_MOTION_STATUS, Action::None)
            .tag_action(
                tags::PARALLEL_REDUCTION_FACTOR_SECOND_IN_PLANE,
                Action::None,
            )
            .tag_action(tags::CARDIAC_BEAT_REJECTION_TECHNIQUE, Action::None)
            .tag_action(
                tags::RESPIRATORY_MOTION_COMPENSATION_TECHNIQUE,
                Action::None,
            )
            .tag_action(tags::RESPIRATORY_SIGNAL_SOURCE, Action::None)
            .tag_action(tags::BULK_MOTION_COMPENSATION_TECHNIQUE, Action::None)
            .tag_action(tags::BULK_MOTION_SIGNAL_SOURCE, Action::None)
            .tag_action(tags::APPLICABLE_SAFETY_STANDARD_AGENCY, Action::None)
            .tag_action(tags::APPLICABLE_SAFETY_STANDARD_DESCRIPTION, Action::None)
            .tag_action(tags::OPERATING_MODE_SEQUENCE, Action::None)
            .tag_action(tags::OPERATING_MODE_TYPE, Action::None)
            .tag_action(tags::OPERATING_MODE, Action::None)
            .tag_action(tags::SPECIFIC_ABSORPTION_RATE_DEFINITION, Action::None)
            .tag_action(tags::GRADIENT_OUTPUT_TYPE, Action::None)
            .tag_action(tags::SPECIFIC_ABSORPTION_RATE_VALUE, Action::None)
            .tag_action(tags::GRADIENT_OUTPUT, Action::None)
            .tag_action(tags::FLOW_COMPENSATION_DIRECTION, Action::None)
            .tag_action(tags::TAGGING_DELAY, Action::None)
            .tag_action(
                tags::CHEMICAL_SHIFT_MINIMUM_INTEGRATION_LIMIT_IN_HZ,
                Action::None,
            )
            .tag_action(
                tags::CHEMICAL_SHIFT_MAXIMUM_INTEGRATION_LIMIT_IN_HZ,
                Action::None,
            )
            .tag_action(tags::MR_VELOCITY_ENCODING_SEQUENCE, Action::None)
            .tag_action(tags::FIRST_ORDER_PHASE_CORRECTION, Action::None)
            .tag_action(tags::WATER_REFERENCED_PHASE_CORRECTION, Action::None)
            .tag_action(tags::MR_SPECTROSCOPY_ACQUISITION_TYPE, Action::None)
            .tag_action(tags::RESPIRATORY_CYCLE_POSITION, Action::None)
            .tag_action(tags::VELOCITY_ENCODING_MAXIMUM_VALUE, Action::None)
            .tag_action(tags::TAG_SPACING_SECOND_DIMENSION, Action::None)
            .tag_action(tags::TAG_ANGLE_SECOND_AXIS, Action::None)
            .tag_action(tags::FRAME_ACQUISITION_DURATION, Action::None)
            .tag_action(tags::MR_IMAGE_FRAME_TYPE_SEQUENCE, Action::None)
            .tag_action(tags::MR_SPECTROSCOPY_FRAME_TYPE_SEQUENCE, Action::None)
            .tag_action(
                tags::MR_ACQUISITION_PHASE_ENCODING_STEPS_IN_PLANE,
                Action::None,
            )
            .tag_action(
                tags::MR_ACQUISITION_PHASE_ENCODING_STEPS_OUT_OF_PLANE,
                Action::None,
            )
            .tag_action(tags::SPECTROSCOPY_ACQUISITION_PHASE_COLUMNS, Action::None)
            .tag_action(tags::CARDIAC_CYCLE_POSITION, Action::None)
            .tag_action(tags::SPECIFIC_ABSORPTION_RATE_SEQUENCE, Action::None)
            .tag_action(tags::CONTRIBUTION_DESCRIPTION, Action::Remove)
            .tag_action(tags::STUDY_INSTANCE_UID, Action::HashUID)
            .tag_action(tags::SERIES_INSTANCE_UID, Action::HashUID)
            .tag_action(tags::STUDY_ID, Action::Empty)
            .tag_action(tags::SERIES_NUMBER, Action::None)
            .tag_action(tags::ACQUISITION_NUMBER, Action::None)
            .tag_action(tags::INSTANCE_NUMBER, Action::None)
            .tag_action(tags::ITEM_NUMBER, Action::None)
            .tag_action(tags::PATIENT_ORIENTATION, Action::None)
            .tag_action(tags::OVERLAY_NUMBER, Action::None)
            .tag_action(tags::CURVE_NUMBER, Action::None)
            .tag_action(tags::LUT_NUMBER, Action::None)
            .tag_action(tags::IMAGE_POSITION, Action::None)
            .tag_action(tags::IMAGE_ORIENTATION, Action::None)
            .tag_action(tags::FRAME_OF_REFERENCE_UID, Action::HashUID)
            .tag_action(tags::LATERALITY, Action::None)
            .tag_action(tags::IMAGE_LATERALITY, Action::None)
            .tag_action(tags::TEMPORAL_POSITION_IDENTIFIER, Action::None)
            .tag_action(tags::NUMBER_OF_TEMPORAL_POSITIONS, Action::None)
            .tag_action(tags::TEMPORAL_RESOLUTION, Action::None)
            .tag_action(
                tags::SYNCHRONIZATION_FRAME_OF_REFERENCE_UID,
                Action::HashUID,
            )
            .tag_action(tags::SERIES_IN_STUDY, Action::None)
            .tag_action(tags::IMAGES_IN_ACQUISITION, Action::None)
            .tag_action(tags::ACQUISITIONS_IN_STUDY, Action::None)
            .tag_action(tags::POSITION_REFERENCE_INDICATOR, Action::None)
            .tag_action(tags::SLICE_LOCATION, Action::None)
            .tag_action(tags::OTHER_STUDY_NUMBERS, Action::None)
            .tag_action(tags::NUMBER_OF_PATIENT_RELATED_STUDIES, Action::None)
            .tag_action(tags::NUMBER_OF_PATIENT_RELATED_SERIES, Action::None)
            .tag_action(tags::NUMBER_OF_PATIENT_RELATED_INSTANCES, Action::None)
            .tag_action(tags::NUMBER_OF_STUDY_RELATED_SERIES, Action::None)
            .tag_action(tags::NUMBER_OF_STUDY_RELATED_INSTANCES, Action::None)
            .tag_action(tags::NUMBER_OF_SERIES_RELATED_INSTANCES, Action::None)
            .tag_action(tags::MODIFYING_DEVICE_ID, Action::Remove)
            .tag_action(tags::MODIFYING_DEVICE_MANUFACTURER, Action::Remove)
            .tag_action(tags::MODIFIED_IMAGE_DESCRIPTION, Action::Remove)
            .tag_action(tags::IMAGE_COMMENTS, Action::Remove)
            .tag_action(tags::STACK_ID, Action::None)
            .tag_action(tags::IN_STACK_POSITION_NUMBER, Action::None)
            .tag_action(tags::FRAME_ANATOMY_SEQUENCE, Action::None)
            .tag_action(tags::FRAME_LATERALITY, Action::None)
            .tag_action(tags::FRAME_CONTENT_SEQUENCE, Action::None)
            .tag_action(tags::PLANE_POSITION_SEQUENCE, Action::None)
            .tag_action(tags::PLANE_ORIENTATION_SEQUENCE, Action::None)
            .tag_action(tags::TEMPORAL_POSITION_INDEX, Action::None)
            .tag_action(tags::NOMINAL_CARDIAC_TRIGGER_DELAY_TIME, Action::None)
            .tag_action(tags::FRAME_ACQUISITION_NUMBER, Action::None)
            .tag_action(tags::DIMENSION_INDEX_VALUES, Action::None)
            .tag_action(tags::FRAME_COMMENTS, Action::None)
            .tag_action(tags::CONCATENATION_UID, Action::HashUID)
            .tag_action(tags::IN_CONCATENATION_NUMBER, Action::None)
            .tag_action(tags::IN_CONCATENATION_TOTAL_NUMBER, Action::None)
            .tag_action(tags::DIMENSION_ORGANIZATION_UID, Action::HashUID)
            .tag_action(tags::DIMENSION_INDEX_POINTER, Action::None)
            .tag_action(tags::FUNCTIONAL_GROUP_POINTER, Action::None)
            .tag_action(tags::DIMENSION_INDEX_PRIVATE_CREATOR, Action::None)
            .tag_action(tags::DIMENSION_ORGANIZATION_SEQUENCE, Action::None)
            .tag_action(tags::DIMENSION_INDEX_SEQUENCE, Action::None)
            .tag_action(tags::CONCATENATION_FRAME_OFFSET_NUMBER, Action::None)
            .tag_action(tags::FUNCTIONAL_GROUP_PRIVATE_CREATOR, Action::None)
            .tag_action(tags::SAMPLES_PER_PIXEL, Action::None)
            .tag_action(tags::PHOTOMETRIC_INTERPRETATION, Action::None)
            .tag_action(tags::PLANAR_CONFIGURATION, Action::None)
            .tag_action(tags::NUMBER_OF_FRAMES, Action::None)
            .tag_action(tags::FRAME_INCREMENT_POINTER, Action::None)
            .tag_action(tags::ROWS, Action::None)
            .tag_action(tags::COLUMNS, Action::None)
            .tag_action(tags::PLANES, Action::None)
            .tag_action(tags::ULTRASOUND_COLOR_DATA_PRESENT, Action::None)
            .tag_action(tags::PIXEL_SPACING, Action::None)
            .tag_action(tags::ZOOM_FACTOR, Action::None)
            .tag_action(tags::ZOOM_CENTER, Action::None)
            .tag_action(tags::PIXEL_ASPECT_RATIO, Action::None)
            .tag_action(tags::CORRECTED_IMAGE, Action::None)
            .tag_action(tags::BITS_ALLOCATED, Action::None)
            .tag_action(tags::BITS_STORED, Action::None)
            .tag_action(tags::HIGH_BIT, Action::None)
            .tag_action(tags::PIXEL_REPRESENTATION, Action::None)
            .tag_action(tags::SMALLEST_IMAGE_PIXEL_VALUE, Action::None)
            .tag_action(tags::LARGEST_IMAGE_PIXEL_VALUE, Action::None)
            .tag_action(tags::SMALLEST_PIXEL_VALUE_IN_SERIES, Action::None)
            .tag_action(tags::LARGEST_PIXEL_VALUE_IN_SERIES, Action::None)
            .tag_action(tags::SMALLEST_IMAGE_PIXEL_VALUE_IN_PLANE, Action::None)
            .tag_action(tags::LARGEST_IMAGE_PIXEL_VALUE_IN_PLANE, Action::None)
            .tag_action(tags::PIXEL_PADDING_VALUE, Action::None)
            .tag_action(tags::QUALITY_CONTROL_IMAGE, Action::None)
            .tag_action(tags::BURNED_IN_ANNOTATION, Action::None)
            .tag_action(tags::PIXEL_INTENSITY_RELATIONSHIP, Action::None)
            .tag_action(tags::PIXEL_INTENSITY_RELATIONSHIP_SIGN, Action::None)
            .tag_action(tags::WINDOW_CENTER, Action::None)
            .tag_action(tags::WINDOW_WIDTH, Action::None)
            .tag_action(tags::RESCALE_INTERCEPT, Action::None)
            .tag_action(tags::RESCALE_SLOPE, Action::None)
            .tag_action(tags::RESCALE_TYPE, Action::None)
            .tag_action(tags::WINDOW_CENTER_WIDTH_EXPLANATION, Action::None)
            .tag_action(tags::RECOMMENDED_VIEWING_MODE, Action::None)
            .tag_action(
                tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
                Action::None,
            )
            .tag_action(
                tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
                Action::None,
            )
            .tag_action(
                tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
                Action::None,
            )
            .tag_action(tags::PALETTE_COLOR_LOOKUP_TABLE_UID, Action::HashUID)
            .tag_action(tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA, Action::None)
            .tag_action(tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA, Action::None)
            .tag_action(tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA, Action::None)
            .tag_action(tags::LARGE_PALETTE_COLOR_LOOKUP_TABLE_UID, Action::HashUID)
            .tag_action(
                tags::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA,
                Action::None,
            )
            .tag_action(
                tags::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA,
                Action::None,
            )
            .tag_action(
                tags::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA,
                Action::None,
            )
            .tag_action(tags::BREAST_IMPLANT_PRESENT, Action::None)
            .tag_action(tags::PARTIAL_VIEW, Action::None)
            .tag_action(tags::PARTIAL_VIEW_DESCRIPTION, Action::None)
            .tag_action(tags::LOSSY_IMAGE_COMPRESSION, Action::None)
            .tag_action(tags::LOSSY_IMAGE_COMPRESSION_RATIO, Action::None)
            .tag_action(tags::MODALITY_LUT_SEQUENCE, Action::None)
            .tag_action(tags::LUT_DESCRIPTOR, Action::None)
            .tag_action(tags::LUT_EXPLANATION, Action::None)
            .tag_action(tags::MODALITY_LUT_TYPE, Action::None)
            .tag_action(tags::LUT_DATA, Action::None)
            .tag_action(tags::VOILUT_SEQUENCE, Action::None)
            .tag_action(tags::SOFTCOPY_VOILUT_SEQUENCE, Action::None)
            .tag_action(tags::IMAGE_PRESENTATION_COMMENTS, Action::Remove)
            .tag_action(tags::BI_PLANE_ACQUISITION_SEQUENCE, Action::None)
            .tag_action(tags::REPRESENTATIVE_FRAME_NUMBER, Action::None)
            .tag_action(tags::FRAME_NUMBERS_OF_INTEREST, Action::None)
            .tag_action(tags::FRAME_OF_INTEREST_DESCRIPTION, Action::None)
            .tag_action(tags::MASK_POINTERS, Action::None)
            .tag_action(tags::MASK_SUBTRACTION_SEQUENCE, Action::None)
            .tag_action(tags::MASK_OPERATION, Action::None)
            .tag_action(tags::APPLICABLE_FRAME_RANGE, Action::None)
            .tag_action(tags::MASK_FRAME_NUMBERS, Action::None)
            .tag_action(tags::CONTRAST_FRAME_AVERAGING, Action::None)
            .tag_action(tags::MASK_SUB_PIXEL_SHIFT, Action::None)
            .tag_action(tags::TID_OFFSET, Action::None)
            .tag_action(tags::MASK_OPERATION_EXPLANATION, Action::None)
            .tag_action(tags::DATA_POINT_ROWS, Action::None)
            .tag_action(tags::DATA_POINT_COLUMNS, Action::None)
            .tag_action(tags::SIGNAL_DOMAIN_COLUMNS, Action::None)
            .tag_action(tags::LARGEST_MONOCHROME_PIXEL_VALUE, Action::None)
            .tag_action(tags::DATA_REPRESENTATION, Action::None)
            .tag_action(tags::PIXEL_MEASURES_SEQUENCE, Action::None)
            .tag_action(tags::FRAME_VOILUT_SEQUENCE, Action::None)
            .tag_action(tags::PIXEL_VALUE_TRANSFORMATION_SEQUENCE, Action::None)
            .tag_action(tags::SIGNAL_DOMAIN_ROWS, Action::None)
            .tag_action(tags::STUDY_STATUS_ID, Action::None)
            .tag_action(tags::STUDY_PRIORITY_ID, Action::None)
            .tag_action(tags::STUDY_ID_ISSUER, Action::Remove)
            .tag_action(tags::STUDY_VERIFIED_DATE, Action::HashDate)
            .tag_action(tags::STUDY_VERIFIED_TIME, Action::None)
            .tag_action(tags::STUDY_READ_DATE, Action::HashDate)
            .tag_action(tags::STUDY_READ_TIME, Action::None)
            .tag_action(tags::SCHEDULED_STUDY_START_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_STUDY_START_TIME, Action::None)
            .tag_action(tags::SCHEDULED_STUDY_STOP_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_STUDY_STOP_TIME, Action::None)
            .tag_action(tags::SCHEDULED_STUDY_LOCATION, Action::Remove)
            .tag_action(tags::SCHEDULED_STUDY_LOCATION_AE_TITLE, Action::Remove)
            .tag_action(tags::REASON_FOR_STUDY, Action::Remove)
            .tag_action(tags::REQUESTING_PHYSICIAN, Action::Remove)
            .tag_action(tags::REQUESTING_SERVICE, Action::Remove)
            .tag_action(tags::STUDY_ARRIVAL_DATE, Action::HashDate)
            .tag_action(tags::STUDY_ARRIVAL_TIME, Action::None)
            .tag_action(tags::STUDY_COMPLETION_DATE, Action::HashDate)
            .tag_action(tags::STUDY_COMPLETION_TIME, Action::None)
            .tag_action(tags::STUDY_COMPONENT_STATUS_ID, Action::None)
            .tag_action(tags::REQUESTED_PROCEDURE_DESCRIPTION, Action::Remove)
            .tag_action(tags::REQUESTED_PROCEDURE_CODE_SEQUENCE, Action::None)
            .tag_action(tags::REQUESTED_CONTRAST_AGENT, Action::Remove)
            .tag_action(tags::STUDY_COMMENTS, Action::Remove)
            .tag_action(tags::REFERENCED_PATIENT_ALIAS_SEQUENCE, Action::None)
            .tag_action(tags::VISIT_STATUS_ID, Action::None)
            .tag_action(tags::ADMISSION_ID, Action::Remove)
            .tag_action(tags::ISSUER_OF_ADMISSION_ID, Action::Remove)
            .tag_action(tags::ROUTE_OF_ADMISSIONS, Action::None)
            .tag_action(tags::SCHEDULED_ADMISSION_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_ADMISSION_TIME, Action::None)
            .tag_action(tags::SCHEDULED_DISCHARGE_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_DISCHARGE_TIME, Action::None)
            .tag_action(tags::ADMITTING_DATE, Action::Remove)
            .tag_action(tags::ADMITTING_TIME, Action::Remove)
            .tag_action(tags::DISCHARGE_DATE, Action::HashDate)
            .tag_action(tags::DISCHARGE_TIME, Action::None)
            .tag_action(tags::DISCHARGE_DIAGNOSIS_DESCRIPTION, Action::Remove)
            .tag_action(tags::DISCHARGE_DIAGNOSIS_CODE_SEQUENCE, Action::None)
            .tag_action(tags::SPECIAL_NEEDS, Action::Remove)
            .tag_action(tags::SERVICE_EPISODE_ID, Action::Remove)
            .tag_action(tags::ISSUER_OF_SERVICE_EPISODE_ID, Action::Remove)
            .tag_action(tags::SERVICE_EPISODE_DESCRIPTION, Action::Remove)
            .tag_action(tags::CURRENT_PATIENT_LOCATION, Action::Remove)
            .tag_action(tags::PATIENT_INSTITUTION_RESIDENCE, Action::Remove)
            .tag_action(tags::PATIENT_STATE, Action::Remove)
            .tag_action(tags::REFERENCED_PATIENT_ALIAS_SEQUENCE, Action::Remove)
            .tag_action(tags::VISIT_COMMENTS, Action::Remove)
            .tag_action(tags::WAVEFORM_ORIGINALITY, Action::None)
            .tag_action(tags::NUMBER_OF_WAVEFORM_CHANNELS, Action::None)
            .tag_action(tags::NUMBER_OF_WAVEFORM_SAMPLES, Action::None)
            .tag_action(tags::SAMPLING_FREQUENCY, Action::None)
            .tag_action(tags::MULTIPLEX_GROUP_LABEL, Action::None)
            .tag_action(tags::CHANNEL_DEFINITION_SEQUENCE, Action::None)
            .tag_action(tags::WAVEFORM_CHANNEL_NUMBER, Action::None)
            .tag_action(tags::CHANNEL_LABEL, Action::None)
            .tag_action(tags::CHANNEL_STATUS, Action::None)
            .tag_action(tags::CHANNEL_SOURCE_SEQUENCE, Action::None)
            .tag_action(tags::CHANNEL_SOURCE_MODIFIERS_SEQUENCE, Action::None)
            .tag_action(tags::SOURCE_WAVEFORM_SEQUENCE, Action::None)
            .tag_action(tags::CHANNEL_DERIVATION_DESCRIPTION, Action::None)
            .tag_action(tags::CHANNEL_SENSITIVITY, Action::None)
            .tag_action(tags::CHANNEL_SENSITIVITY_UNITS_SEQUENCE, Action::None)
            .tag_action(tags::CHANNEL_SENSITIVITY_CORRECTION_FACTOR, Action::None)
            .tag_action(tags::CHANNEL_BASELINE, Action::None)
            .tag_action(tags::CHANNEL_TIME_SKEW, Action::None)
            .tag_action(tags::CHANNEL_SAMPLE_SKEW, Action::None)
            .tag_action(tags::CHANNEL_OFFSET, Action::None)
            .tag_action(tags::WAVEFORM_BITS_STORED, Action::None)
            .tag_action(tags::FILTER_LOW_FREQUENCY, Action::None)
            .tag_action(tags::FILTER_HIGH_FREQUENCY, Action::None)
            .tag_action(tags::NOTCH_FILTER_FREQUENCY, Action::None)
            .tag_action(tags::NOTCH_FILTER_BANDWIDTH, Action::None)
            .tag_action(tags::SCHEDULED_STATION_AE_TITLE, Action::Remove)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_START_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_START_TIME, Action::None)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_END_DATE, Action::HashDate)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_END_TIME, Action::None)
            .tag_action(tags::SCHEDULED_PERFORMING_PHYSICIAN_NAME, Action::Remove)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_DESCRIPTION, Action::Remove)
            .tag_action(tags::SCHEDULED_PROTOCOL_CODE_SEQUENCE, Action::None)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_ID, Action::None)
            .tag_action(
                tags::SCHEDULED_PERFORMING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::SCHEDULED_STATION_NAME, Action::Remove)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_LOCATION, Action::Remove)
            .tag_action(tags::PRE_MEDICATION, Action::Remove)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_STATUS, Action::None)
            .tag_action(tags::SCHEDULED_PROCEDURE_STEP_SEQUENCE, Action::None)
            .tag_action(
                tags::REFERENCED_NON_IMAGE_COMPOSITE_SOP_INSTANCE_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::PERFORMED_STATION_AE_TITLE, Action::Remove)
            .tag_action(tags::PERFORMED_STATION_NAME, Action::Remove)
            .tag_action(tags::PERFORMED_LOCATION, Action::Remove)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_START_DATE, Action::HashDate)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_START_TIME, Action::None)
            .tag_action(tags::PERFORMED_STATION_NAME_CODE_SEQUENCE, Action::Remove)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_END_DATE, Action::HashDate)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_END_TIME, Action::None)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_ID, Action::Remove)
            .tag_action(tags::PERFORMED_PROCEDURE_STEP_DESCRIPTION, Action::Remove)
            .tag_action(tags::PERFORMED_PROCEDURE_TYPE_DESCRIPTION, Action::None)
            .tag_action(tags::PERFORMED_PROTOCOL_CODE_SEQUENCE, Action::None)
            .tag_action(tags::SCHEDULED_STEP_ATTRIBUTES_SEQUENCE, Action::None)
            .tag_action(tags::REQUEST_ATTRIBUTES_SEQUENCE, Action::Remove)
            .tag_action(
                tags::COMMENTS_ON_THE_PERFORMED_PROCEDURE_STEP,
                Action::Remove,
            )
            .tag_action(
                tags::PERFORMED_PROCEDURE_STEP_DISCONTINUATION_REASON_CODE_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::QUANTITY_SEQUENCE, Action::None)
            .tag_action(tags::QUANTITY, Action::None)
            .tag_action(tags::MEASURING_UNITS_SEQUENCE, Action::None)
            .tag_action(tags::BILLING_ITEM_SEQUENCE, Action::None)
            .tag_action(tags::TOTAL_TIME_OF_FLUOROSCOPY, Action::None)
            .tag_action(tags::TOTAL_NUMBER_OF_EXPOSURES, Action::None)
            .tag_action(tags::ENTRANCE_DOSE, Action::None)
            .tag_action(tags::EXPOSED_AREA, Action::None)
            .tag_action(tags::DISTANCE_SOURCE_TO_ENTRANCE, Action::None)
            .tag_action(tags::DISTANCE_SOURCE_TO_SUPPORT, Action::None)
            .tag_action(tags::EXPOSURE_DOSE_SEQUENCE, Action::None)
            .tag_action(tags::COMMENTS_ON_RADIATION_DOSE, Action::None)
            .tag_action(tags::X_RAY_OUTPUT, Action::None)
            .tag_action(tags::HALF_VALUE_LAYER, Action::None)
            .tag_action(tags::ORGAN_DOSE, Action::None)
            .tag_action(tags::ORGAN_EXPOSED, Action::None)
            .tag_action(tags::BILLING_PROCEDURE_STEP_SEQUENCE, Action::None)
            .tag_action(tags::FILM_CONSUMPTION_SEQUENCE, Action::None)
            .tag_action(tags::BILLING_SUPPLIES_AND_DEVICES_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_PROCEDURE_STEP_SEQUENCE, Action::None)
            .tag_action(tags::PERFORMED_SERIES_SEQUENCE, Action::None)
            .tag_action(tags::COMMENTS_ON_THE_SCHEDULED_PROCEDURE_STEP, Action::None)
            .tag_action(tags::SPECIMEN_ACCESSION_NUMBER, Action::None)
            .tag_action(tags::SPECIMEN_SEQUENCE, Action::None)
            .tag_action(tags::SPECIMEN_IDENTIFIER, Action::None)
            .tag_action(tags::ACQUISITION_CONTEXT_SEQUENCE, Action::Remove)
            .tag_action(tags::ACQUISITION_CONTEXT_DESCRIPTION, Action::None)
            .tag_action(tags::SPECIMEN_TYPE_CODE_SEQUENCE, Action::None)
            .tag_action(tags::SLIDE_IDENTIFIER, Action::None)
            .tag_action(tags::IMAGE_CENTER_POINT_COORDINATES_SEQUENCE, Action::None)
            .tag_action(tags::X_OFFSET_IN_SLIDE_COORDINATE_SYSTEM, Action::None)
            .tag_action(tags::Y_OFFSET_IN_SLIDE_COORDINATE_SYSTEM, Action::None)
            .tag_action(tags::Z_OFFSET_IN_SLIDE_COORDINATE_SYSTEM, Action::None)
            .tag_action(tags::PIXEL_SPACING_SEQUENCE, Action::None)
            .tag_action(tags::COORDINATE_SYSTEM_AXIS_CODE_SEQUENCE, Action::None)
            .tag_action(tags::MEASUREMENT_UNITS_CODE_SEQUENCE, Action::None)
            .tag_action(tags::REQUESTED_PROCEDURE_ID, Action::None)
            .tag_action(tags::REASON_FOR_THE_REQUESTED_PROCEDURE, Action::None)
            .tag_action(tags::REQUESTED_PROCEDURE_PRIORITY, Action::None)
            .tag_action(tags::PATIENT_TRANSPORT_ARRANGEMENTS, Action::Remove)
            .tag_action(tags::REQUESTED_PROCEDURE_LOCATION, Action::Remove)
            .tag_action(tags::CONFIDENTIALITY_CODE, Action::None)
            .tag_action(tags::REPORTING_PRIORITY, Action::None)
            .tag_action(
                tags::NAMES_OF_INTENDED_RECIPIENTS_OF_RESULTS,
                Action::Remove,
            )
            .tag_action(
                tags::INTENDED_RECIPIENTS_OF_RESULTS_IDENTIFICATION_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::PERSON_ADDRESS, Action::Remove)
            .tag_action(tags::PERSON_TELEPHONE_NUMBERS, Action::Remove)
            .tag_action(tags::REQUESTED_PROCEDURE_COMMENTS, Action::Remove)
            .tag_action(tags::REASON_FOR_THE_IMAGING_SERVICE_REQUEST, Action::Remove)
            .tag_action(
                tags::ISSUE_DATE_OF_IMAGING_SERVICE_REQUEST,
                Action::HashDate,
            )
            .tag_action(tags::ISSUE_TIME_OF_IMAGING_SERVICE_REQUEST, Action::None)
            .tag_action(tags::ORDER_ENTERED_BY, Action::Remove)
            .tag_action(tags::ORDER_ENTERER_LOCATION, Action::Remove)
            .tag_action(tags::ORDER_CALLBACK_PHONE_NUMBER, Action::Remove)
            .tag_action(
                tags::PLACER_ORDER_NUMBER_IMAGING_SERVICE_REQUEST,
                Action::Hash { length: Some(16) },
            )
            .tag_action(
                tags::FILLER_ORDER_NUMBER_IMAGING_SERVICE_REQUEST,
                Action::Hash { length: Some(16) },
            )
            .tag_action(tags::IMAGING_SERVICE_REQUEST_COMMENTS, Action::Remove)
            .tag_action(
                tags::CONFIDENTIALITY_CONSTRAINT_ON_PATIENT_DATA_DESCRIPTION,
                Action::Remove,
            )
            .tag_action(
                tags::REFERENCED_GENERAL_PURPOSE_SCHEDULED_PROCEDURE_STEP_TRANSACTION_UID,
                Action::HashUID,
            )
            .tag_action(tags::SCHEDULED_STATION_NAME_CODE_SEQUENCE, Action::Remove)
            .tag_action(
                tags::SCHEDULED_STATION_GEOGRAPHIC_LOCATION_CODE_SEQUENCE,
                Action::Remove,
            )
            .tag_action(
                tags::PERFORMED_STATION_GEOGRAPHIC_LOCATION_CODE_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::SCHEDULED_HUMAN_PERFORMERS_SEQUENCE, Action::Remove)
            .tag_action(tags::ACTUAL_HUMAN_PERFORMERS_SEQUENCE, Action::Remove)
            .tag_action(tags::HUMAN_PERFORMER_ORGANIZATION, Action::Remove)
            .tag_action(tags::HUMAN_PERFORMER_NAME, Action::Remove)
            .tag_action(tags::ENTRANCE_DOSE_INM_GY, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_MAPPING_SEQUENCE, Action::None)
            .tag_action(tags::LUT_LABEL, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_LAST_VALUE_MAPPED, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_LUT_DATA, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_FIRST_VALUE_MAPPED, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_INTERCEPT, Action::None)
            .tag_action(tags::REAL_WORLD_VALUE_SLOPE, Action::None)
            .tag_action(tags::RELATIONSHIP_TYPE, Action::None)
            .tag_action(tags::VERIFYING_ORGANIZATION, Action::Remove)
            .tag_action(tags::VERIFICATION_DATE_TIME, Action::HashDate)
            .tag_action(tags::OBSERVATION_DATE_TIME, Action::HashDate)
            .tag_action(tags::VALUE_TYPE, Action::None)
            .tag_action(tags::CONCEPT_NAME_CODE_SEQUENCE, Action::None)
            .tag_action(tags::CONTINUITY_OF_CONTENT, Action::None)
            .tag_action(tags::VERIFYING_OBSERVER_SEQUENCE, Action::Remove)
            .tag_action(tags::VERIFYING_OBSERVER_NAME, Action::Remove)
            .tag_action(tags::AUTHOR_OBSERVER_SEQUENCE, Action::Remove)
            .tag_action(tags::PARTICIPANT_SEQUENCE, Action::Remove)
            .tag_action(tags::CUSTODIAL_ORGANIZATION_SEQUENCE, Action::Remove)
            .tag_action(
                tags::VERIFYING_OBSERVER_IDENTIFICATION_CODE_SEQUENCE,
                Action::Remove,
            )
            .tag_action(tags::REFERENCED_WAVEFORM_CHANNELS, Action::None)
            .tag_action(tags::DATE_TIME, Action::HashDate)
            .tag_action(tags::DATE, Action::HashDate)
            .tag_action(tags::TIME, Action::None)
            .tag_action(tags::PERSON_NAME, Action::Remove)
            .tag_action(tags::UID, Action::HashUID)
            .tag_action(tags::TEMPORAL_RANGE_TYPE, Action::None)
            .tag_action(tags::REFERENCED_SAMPLE_POSITIONS, Action::None)
            .tag_action(tags::REFERENCED_FRAME_NUMBERS, Action::None)
            .tag_action(tags::REFERENCED_TIME_OFFSETS, Action::None)
            .tag_action(tags::REFERENCED_DATE_TIME, Action::HashDate)
            .tag_action(tags::TEXT_VALUE, Action::None)
            .tag_action(tags::CONCEPT_CODE_SEQUENCE, Action::None)
            .tag_action(tags::ANNOTATION_GROUP_NUMBER, Action::None)
            .tag_action(tags::MODIFIER_CODE_SEQUENCE, Action::None)
            .tag_action(tags::MEASURED_VALUE_SEQUENCE, Action::None)
            .tag_action(tags::NUMERIC_VALUE, Action::None)
            .tag_action(tags::PREDECESSOR_DOCUMENTS_SEQUENCE, Action::None)
            .tag_action(tags::REFERENCED_REQUEST_SEQUENCE, Action::None)
            .tag_action(tags::PERFORMED_PROCEDURE_CODE_SEQUENCE, Action::None)
            .tag_action(
                tags::CURRENT_REQUESTED_PROCEDURE_EVIDENCE_SEQUENCE,
                Action::None,
            )
            .tag_action(tags::PERTINENT_OTHER_EVIDENCE_SEQUENCE, Action::None)
            .tag_action(tags::COMPLETION_FLAG, Action::None)
            .tag_action(tags::VERIFICATION_FLAG, Action::None)
            .tag_action(tags::CONTENT_TEMPLATE_SEQUENCE, Action::None)
            .tag_action(tags::IDENTICAL_DOCUMENTS_SEQUENCE, Action::None)
            .tag_action(tags::CONTENT_SEQUENCE, Action::Remove)
            .tag_action(tags::WAVEFORM_ANNOTATION_SEQUENCE, Action::None)
            .tag_action(tags::TEMPLATE_VERSION, Action::None)
            .tag_action(tags::TEMPLATE_LOCAL_VERSION, Action::None)
            .tag_action(tags::TEMPLATE_EXTENSION_FLAG, Action::None)
            .tag_action(tags::TEMPLATE_EXTENSION_ORGANIZATION_UID, Action::HashUID)
            .tag_action(tags::TEMPLATE_EXTENSION_CREATOR_UID, Action::HashUID)
            .tag_action(tags::REFERENCED_CONTENT_ITEM_IDENTIFIER, Action::None)
            .tag_action(tags::FIDUCIAL_UID, Action::HashUID)
            .tag_action(tags::STORAGE_MEDIA_FILE_SET_UID, Action::HashUID)
            .tag_action(tags::ICON_IMAGE_SEQUENCE, Action::Remove)
            .tag_action(tags::TOPIC_SUBJECT, Action::Remove)
            .tag_action(tags::TOPIC_AUTHOR, Action::Remove)
            .tag_action(tags::TOPIC_KEYWORDS, Action::Remove)
            .tag_action(tags::DIGITAL_SIGNATURE_UID, Action::HashUID)
            .tag_action(tags::TEXT_STRING, Action::Remove)
            .tag_action(tags::REFERENCED_FRAME_OF_REFERENCE_UID, Action::HashUID)
            .tag_action(tags::RELATED_FRAME_OF_REFERENCE_UID, Action::HashUID)
            .tag_action(tags::DOSE_REFERENCE_UID, Action::HashUID)
            .tag_action(tags::ARBITRARY, Action::Remove)
            .tag_action(tags::TEXT_COMMENTS, Action::Remove)
            .tag_action(tags::RESULTS_ID_ISSUER, Action::Remove)
            .tag_action(tags::INTERPRETATION_RECORDER, Action::Remove)
            .tag_action(tags::INTERPRETATION_TRANSCRIBER, Action::Remove)
            .tag_action(tags::INTERPRETATION_TEXT, Action::Remove)
            .tag_action(tags::INTERPRETATION_AUTHOR, Action::Remove)
            .tag_action(tags::INTERPRETATION_APPROVER_SEQUENCE, Action::Remove)
            .tag_action(tags::PHYSICIAN_APPROVING_INTERPRETATION, Action::Remove)
            .tag_action(tags::INTERPRETATION_DIAGNOSIS_DESCRIPTION, Action::Remove)
            .tag_action(tags::RESULTS_DISTRIBUTION_LIST_SEQUENCE, Action::Remove)
            .tag_action(tags::DISTRIBUTION_NAME, Action::Remove)
            .tag_action(tags::DISTRIBUTION_ADDRESS, Action::Remove)
            .tag_action(tags::INTERPRETATION_ID_ISSUER, Action::Remove)
            .tag_action(tags::IMPRESSIONS, Action::Remove)
            .tag_action(tags::RESULTS_COMMENTS, Action::Remove)
            .tag_action(tags::DIGITAL_SIGNATURES_SEQUENCE, Action::Remove)
            .tag_action(tags::DATA_SET_TRAILING_PADDING, Action::Remove)
    }
}
