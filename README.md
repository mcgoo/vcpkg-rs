# vcpkg-rs [![Build Status](https://travis-ci.org/mcgoo/vcpkg-rs.svg?branch=master)](https://travis-ci.org/mcgoo/vcpkg-rs) [![Appveyor Build status](https://ci.appveyor.com/api/projects/status/xlqckr07wv0puh3u?svg=true)](https://ci.appveyor.com/project/mcgoo/vcpkg-rs/branch/master)

[Documentation](https://docs.rs/vcpkg) [Changelog](CHANGELOG.md)

This is a helper for finding libraries in a [Vcpkg](https://github.com/Microsoft/vcpkg) installation from cargo build scripts. It works similarly to [pkg-config](https://github.com/alexcrichton/pkg-config-rs). It works on Windows (MSVC ABI), Linux and MacOS.

## Example

Find the library named `foo` in a [Vcpkg](https://github.com/Microsoft/vcpkg) installation and emit cargo metadata to link it:

```rust
extern crate vcpkg;

fn main() {
    vcpkg::find_package("foo").unwrap();
}
```

See the crate [documentation](https://docs.rs/vcpkg) for more information.

## License

See LICENSE-APACHE, and LICENSE-MIT for details.