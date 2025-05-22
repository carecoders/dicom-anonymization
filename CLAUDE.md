# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Development
- `just build` - Build release version
- `just test` - Run tests
- `just clippy` - Run linter
- `cargo deny check` - Check licenses and vulnerabilities

### Python Bindings
- `just build-python` - Build Python bindings for development (uses maturin develop)
- `just test-python` - Build and test Python bindings (installs dependencies with uv)
- `just build-python-release` - Build Python wheels for release

### Documentation
- `just docs` - Generate documentation

## Architecture

This is a DICOM anonymization library written in Rust with Python bindings. The project consists of:

### Core Library (`dicom-anonymization/`)
- **Actions System**: Different anonymization actions (`actions/` module) including Empty, Hash, HashDate, HashUID, Keep, Remove, Replace
- **Configuration**: Builder pattern for creating anonymization configs (`config/` module) with tag-specific actions and policies for private tags, curves, overlays
- **Processor**: Applies configured actions to DICOM elements (`processor.rs`)
- **Anonymizer**: Main API entry point that orchestrates the anonymization process

### Python Bindings (`bindings/python/`)
- Uses maturin to build Python extension from Rust code
- Provides Python API for the core anonymization functionality

### Key Components
- **Config Builder**: Fluent API for building anonymization configurations with tag-specific actions
- **Tag Actions**: Map DICOM tags to specific anonymization actions (remove, hash, replace, etc.)
- **UID Root**: Configurable root for generating new UIDs during anonymization
- **Hash Functions**: Pluggable hash functions for consistent anonymization

### Binary (`dcmanon`)
- CLI tool for anonymizing DICOM files
- Supports single files, directories, and configuration files
- Can create and customize anonymization configurations

The architecture prioritizes performance, type safety, and configurability while providing both library and CLI interfaces.
