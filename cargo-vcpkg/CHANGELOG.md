# Changelog for cargo-vcpkg

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html) as implemented by Cargo.

## [0.1.2] - 2020-06-01

### Fixed

- Fixed help when running under cargo.
- Added pull a step when using a branch so the branch tracks the remote  
  correctly.

### Added

- Display names of packages as they are being built.
- Added some more useful information to the error that is displayed
  if there is no vcpkg metadata in the root crate.

## [0.1.1] - 2020-06-01

### Fixed

- Fixed building from a cmd.exe shell on Windows.

## [0.1.0] - 2020-05-31

### Added

- Initial release
