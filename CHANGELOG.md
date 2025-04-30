# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/carecoders/dicom-anonymization/compare/v0.1.2...v0.2.0) - 2025-04-30

### Added

- *(bin)* display total processing time and file count

### Other

- Remove redundant imports
- Only use `Action::Keep` for keeping tags when their group is removed
- Make Config (de-)serializable
- Update .gitignore
- improve error handling for hash date processing
- *(actions)* migrate from ProcessorError to dedicated ActionError
- use a modular structure for actions
- update allowed licenses in deny.toml
- Merge pull request #6 from carecoders/add-installation-instructions
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
