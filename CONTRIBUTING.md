# Contributing

We welcome and encourage third-party contributions to `dicom-anonymization`, be it reports of issues encountered while using the software or proposals of patches.

## Bug reports

Bugs and other problems should be reported on [GitHub issues](https://github.com/carecoders/dicom-anonymization/issues).

If you report a bug, please:

* Check that it's not already reported in the [GitHub issues](https://github.com/carecoders/dicom-anonymization/issues).
* Provide information to help us diagnose and ideally reproduce the bug.

## Patches

We encourage you to fix a bug via a [GitHub pull request](https://github.com/carecoders/dicom-anonymization/pulls), preferably after creating a related issue and referring it in the PR.

If you contribute code and submit a patch, please note the following:

* We use Rust's stable branch for developing `dicom-anonymization`.
* Pull requests should target the `main` branch.
* Try to follow the established Rust [style guidelines](https://doc.rust-lang.org/1.0.0/style/).

Also please make sure to create new unit tests covering your code additions. You can execute the tests by running:

```bash
cargo test
```
