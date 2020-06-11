# Changelog for cargo-vcpkg

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html) as implemented by Cargo.

## [0.1.5] - 2020-06-10

### Fixes

- Fixes 'dev-dependencies' key handling for 'target' settings.

## [0.1.4] - 2020-06-10

### Added

- Added 'dev-dependencies' key in [package.metadata.vcpkg] to allow installing
  additional packages for development and testing without requiring the end user to
  build those packages.

### Changed

- Changed 'install' key in [package.metadata.vcpkg] to 'dependencies'. ('install' continues to work.)

## [0.1.3] - 2020-06-04

### Added

- On macOS, if Apple clang 11 or later is available, try to bootstrap vcpkg with it first. If this fails, the build will be retried, which will require another compiler.

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
