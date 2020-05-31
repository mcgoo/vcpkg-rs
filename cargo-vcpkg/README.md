# cargo-vcpkg [![Windows](https://github.com/mcgoo/vcpkg-rs/workflows/Windows/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3AWindows) [![macOS](https://github.com/mcgoo/vcpkg-rs/workflows/macOS/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3AmacOS) [![Linux](https://github.com/mcgoo/vcpkg-rs/workflows/Linux/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3ALinux)

[Changelog](CHANGELOG.md)

This command `cargo vcpkg` will create a vcpkg tree and install the packages specified in `Cargo.toml` files in the crate being built and crates it depends on.

## Example

```toml
[package.metadata.vcpkg]
git = "https://github.com/microsoft/vcpkg"
rev = "4c1db68"
install = ["pkg1", "pkg2"]
```

```sh
$ cargo install cargo-vcpkg
$ cargo vcpkg build
    Fetching vcpkg
    Checkout rev/tag/branch 4c1db68
   Compiling pkg1, pkg2
    Finished in 1.93s
$ cargo build
[...]
```

## Per target configuration

It is also possible to install different sets of packages per target, and override the vcpkg triplet to install.

```toml
[package.metadata.vcpkg]
git = "https://github.com/microsoft/vcpkg"
rev = "4c1db68"

[package.metadata.vcpkg.target]
x86_64-apple-darwin = { install = ["sdl2"] }
x86_64-unknown-linux-gnu = { install = ["sdl2"] }
x86_64-pc-windows-msvc = { triplet = "x64-windows-static", install = ["sdl2"] }
```

## License

See LICENSE-APACHE, and LICENSE-MIT for details.
