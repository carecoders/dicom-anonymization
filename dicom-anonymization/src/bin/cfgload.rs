use dicom_anonymization::config::Config;

// Test function to load the config from JSON and print it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_content = r#"{}"#;

    let config = serde_json::from_str::<Config>(&json_content)?;

    println!("{:?}", config);
    // Config {
    //   hash_fn: 0x10448bde4,
    //   uid_root: UidRoot(""),      // should be `None`
    //   remove_private_tags: false, // should be `None`
    //   remove_curves: false,       // should be `None`
    //   remove_overlays: false,     // should be `None`
    //   tag_actions: TagActionMap({})
    // }

    Ok(())
}
