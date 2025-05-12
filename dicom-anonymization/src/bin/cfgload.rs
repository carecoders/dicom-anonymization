use dicom_anonymization::config::Config;

// Test function to load the config from JSON and print it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{}"#;

    let config: Config = serde_json::from_str(json)?;

    println!("{:?}", config);
    // Config {
    //   hash_fn: 0x10448bde4,
    //   uid_root: UidRoot(""),      // should be `None`
    //   remove_private_tags: None,
    //   remove_curves: None,
    //   remove_overlays: false,     // should be `None`
    //   tag_actions: TagActionMap({})
    // }

    Ok(())
}
