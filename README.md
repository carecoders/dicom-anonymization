# dicom-anonymization

[![crates.io](https://img.shields.io/crates/v/dicom-anonymization.svg)](https://crates.io/crates/dicom-anonymization)
[![CI](https://github.com/carecoders/dicom-anonymization/actions/workflows/ci.yml/badge.svg)](https://github.com/carecoders/dicom-anonymization/actions/workflows/ci.yml)
[![Documentation](https://docs.rs/dicom-anonymization/badge.svg)](https://docs.rs/dicom-anonymization)

This repository provides a Rust library and executable for anonymizing [DICOM](https://dicomstandard.org/) files.
With bindings for Python and WebAssembly.

## Features

- **High performance**: optimized Rust implementation for fast DICOM file processing
- **Memory safety**: leverages Rust for memory safety and preventing runtime errors
- **Standards compliant**: follows DICOM anonymization best practices by default
- **Flexible configuration**: comprehensive configuration options
- **Multi-language support**: native Rust library with Python and WebAssembly bindings
- **CLI tool**: command-line tool (`dcmanon`) for batch processing and automation
- **Cross-platform**: works on Windows, MacOS, and Linux

## Usage

### Python

See documentation on [PyPI](https://pypi.org/project/dcmanon).

### Rust

#### Binary

##### Installation

To install the `dcmanon` binary, do this:

```bash
cargo install dicom-anonymization
```

##### Usage

###### anonymize

```bash
$ dcmanon anonymize --help
Anonymize DICOM files

Usage: dcmanon anonymize [OPTIONS] --input <INPUT_PATH> --output <OUTPUT_PATH>

Options:
  -i, --input <INPUT_PATH>    Input file ('-' for stdin) or directory
  -o, --output <OUTPUT_PATH>  Output file ('-' for stdout) or directory
  -c, --config <CONFIG_FILE>  Path to config JSON file
  -u, --uid-root <UID_ROOT>   UID root (default '9999')
      --exclude <TAGS>        Tags to exclude from anonymization, e.g. '00100020,00080050'
  -r, --recursive             Recursively look for files in input directory
      --continue              Continue when file found is not DICOM
  -v, --verbose               Show more verbose output
  -d, --debug                 Show debug output
  -h, --help                  Print help
  -V, --version               Print version
```

**Example**

```bash
dcmanon -i tests/data/test.dcm -o anonymized.dcm
```

###### config create

```bash
$ dcmanon config create --help
Create a configuration file

Usage: dcmanon config create [OPTIONS]

Options:
  -o, --output <CONFIG_FILE>  Path to save the config file  (‘-’ or omitted → stdout) [default: -]
  -v, --verbose               Show more verbose output
  -d, --debug                 Show debug output
  -u, --uid-root <UID_ROOT>   UID root to use
      --exclude <TAGS>        Tags to exclude from anonymization, e.g. '00100020,00080050'
      --diff-only             Only output the dfferences with the default config
  -h, --help                  Print help
```

**Example**

```bash
dcmanon config create -o config_default.json
```

#### Library

See documentation on [docs.rs](https://docs.rs/dicom-anonymization).

##### Installation

To add the library to your project, do this:

```bash
cargo add dicom-anonymization
```

##### Anonymization actions

The library provides several anonymization actions that can be applied to DICOM elements:

- **Empty** - empties element values while preserving the element structure
- **Hash** - anonymizes values using cryptographic hashing with optional length constraints
- **HashDate** - shifts dates based on hash of the `PatientID` element to preserve temporal relationships
- **HashUID** - generates new UIDs from hashes, with an optional UID root
- **Keep** - preserves elements unchanged (useful for keeping certain data or private tags)
- **None** - explicit no-op action (functionally equivalent to `Keep`)
- **Remove** - completely removes elements from the dataset
- **Replace** - substitutes element values with given replacement values

These actions can be configured per-tag to create flexible anonymization policies that balance privacy protection with data utility.

##### Using default configuration

```rust
use std::fs::File;
use dicom_anonymization::Anonymizer;

let file = File::open("tests/data/test.dcm")?;

let anonymizer = Anonymizer::default();
let result = anonymizer.anonymize(file)?;

let output_file = File::create("anonymized.dcm")?;
result.write(output_file)?;
```

##### Using custom configuration

```rust
use std::fs::File;
use dicom_anonymization::tags;
use dicom_anonymization::Anonymizer;
use dicom_anonymization::actions::Action;
use dicom_anonymization::config::builder::ConfigBuilder;
use dicom_anonymization::processor::DefaultProcessor;

// default configuration can be customized/overridden
let config = ConfigBuilder::default()
    .uid_root("1.2.840.123".parse().unwrap())
    .remove_private_tags(true)
    .tag_action(tags::PATIENT_NAME, Action::Replace { value: "John Doe".into() })
    .tag_action(tags::PATIENT_ID, Action::Hash { length: Some(16) })
    .tag_action(tags::ACCESSION_NUMBER, Action::Hash { length: Some(16) })
    .tag_action(tags::STUDY_DATE, Action::HashDate)
    .tag_action(tags::SERIES_DATE, Action::Remove)
    .tag_action(tags::STUDY_INSTANCE_UID, Action::HashUID)
    .tag_action(tags::SERIES_INSTANCE_UID, Action::HashUID)
    .tag_action(tags::SOP_INSTANCE_UID, Action::HashUID)
    .build();

let processor = DefaultProcessor::new(config);
let anonymizer = Anonymizer::new(processor);

let file = File::open("tests/data/test.dcm")?;
let result = anonymizer.anonymize(file)?;

let mut output = Vec::<u8>::new();
result.write(&mut output)?;
```

##### Building configuration from scratch

```rust
use dicom_anonymization::tags;
use dicom_anonymization::actions::Action;
use dicom_anonymization::config::builder::ConfigBuilder;

let config_from_scratch = ConfigBuilder::new()
    .uid_root("1.2.840.123".parse().unwrap())
    .remove_private_tags(false)
    .tag_action(tags::PATIENT_NAME, Action::Replace { value: "John Doe".into() })
    // ...more config rules...
    .build();
```

### WebAssembly

Live demo: [https://carecoders.github.io/dicom-anonymization](https://carecoders.github.io/dicom-anonymization/)

The WebAssembly bindings source code can be found [here](https://github.com/carecoders/dicom-anonymization/tree/main/bindings/wasm).

## Contributing

We welcome contributions from the community. If you are interested in contributing to the project, please read [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
