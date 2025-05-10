use dicom_anonymization::config::builder::ConfigBuilder;
use std::io;
use std::io::Write;

// Convenience function to dump the config to stdout as JSON. Will be integrated into the main program.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::default().build();

    let stdout = io::stdout();
    let mut writer = stdout.lock();
    serde_json::to_writer_pretty(&mut writer, &config)?;

    writeln!(writer)?;

    Ok(())
}
