# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2](https://github.com/carecoders/dicom-anonymization/compare/dicom-anonymization-v0.3.1...dicom-anonymization-v0.3.2) - 2025-06-09

### Added

- *(docs)* add mssing dot to README
- *(docs)* add link to default configuration JSON in README
- *(docs)* add anonymization actions overview to README
- *(bench)* add benchmarking
- *(wasm)* deploy wasm page with github actions to github pages

### Fixed

- *(bench)* add documentation to the benchmarks
- *(bench)* make test data instantiation more efficient
- *(noop)* drastically improve performance of NoopProcessor, add docstring
- *(cli)* remove obsolete cfgdump executable; is now available via `dcmanon config`
- *(wasm)* add non-production warning to page
- *(wasm)* various wasm bindings and demo page improvements
- *(docs)* various README fixes

### Other

- Fix formatting with pre-commit
- Upgrade to edition 2024
- *(actions)* rename DataElementAction trait to ProcessElement
- *(actions)* use trait implementation for ActionProcessor
- *(actions)* replace trait objects with enum dispatch pattern
- *(actions)* add documentation to the action structs
- *(readme)* show example of `dcmanon config create`
- README improvements
- *(wasm)* a couple of docs improvements
- add feature list to README

## [0.3.1](https://github.com/carecoders/dicom-anonymization/compare/dicom-anonymization-v0.3.0...dicom-anonymization-v0.3.1) - 2025-05-23

### Other

- Various Python README improvements

## [0.3.0](https://github.com/carecoders/dicom-anonymization/compare/dicom-anonymization-v0.2.1...dicom-anonymization-v0.3.0) - 2025-05-23

### Other

- Improve Python bindings ([#28](https://github.com/carecoders/dicom-anonymization/pull/28))
- Implement config create sub-command ([#27](https://github.com/carecoders/dicom-anonymization/pull/27))
- Always use default profile for dcmanon ([#26](https://github.com/carecoders/dicom-anonymization/pull/26))
- Simplify HashDate action to always use PatientID as reference tag ([#25](https://github.com/carecoders/dicom-anonymization/pull/25))
- Process sequence items' elements ([#24](https://github.com/carecoders/dicom-anonymization/pull/24))
- Update documentation
- Add Python Anonymizer class ([#20](https://github.com/carecoders/dicom-anonymization/pull/20))

## [0.2.1](https://github.com/carecoders/dicom-anonymization/compare/v0.2.0...v0.2.1) - 2025-05-05

### Added

- add Python bindings
- re-export `dicom_dictionary_std::tags` and `dicom_core::Tag`

## [0.2.0](https://github.com/carecoders/dicom-anonymization/compare/v0.1.2...v0.2.0) - 2025-04-30

### Added

- *(bin)* show total processing time and file count

### Other

- only use `Action::Keep` for keeping tags when their group is removed
- make Config (de-)serializable
- improve error handling for hash date processing
- *(actions)* use dedicated ActionError instead of ProcessorError
- use a modular structure for actions
- update allowed licenses in deny.toml
- add installation instructions to readme

## [0.1.2](https://github.com/carecoders/dicom-anonymization/compare/v0.1.1...v0.1.2) - 2024-11-23

### Other

- add crates.io and docs.rs badges to readme
- add docs.rs link to readme

## [0.1.1](https://github.com/carecoders/dicom-anonymization/releases/tag/v0.1.1) - 2024-11-23

### Fixed

- *(ci)* git lfs pull before running the tests

### Other

- update Cargo.toml with missing fields
- *(pre-commit)* remove unused setup-python action
- add .github/dependabot.yml
- *(ci)* add release-plz to ci
- initial commit
