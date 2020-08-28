# cargo-vcpkg [![Windows](https://github.com/mcgoo/vcpkg-rs/workflows/Windows/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3AWindows) [![macOS](https://github.com/mcgoo/vcpkg-rs/workflows/macOS/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3AmacOS) [![Linux](https://github.com/mcgoo/vcpkg-rs/workflows/Linux/badge.svg?branch=master)](https://github.com/mcgoo/vcpkg-rs/actions?query=workflow%3ALinux)

[Changelog](https://github.com/mcgoo/vcpkg-rs/blob/master/cargo-vcpkg/CHANGELOG.md)

This command `cargo vcpkg` will create a [vcpkg](https://github.com/microsoft/vcpkg) tree and install the packages specified in `Cargo.toml` files in the crate being built and crates it depends on. Crates that use the [vcpkg crate](https://crates.io/crates/vcpkg) will be able to find libraries automatically.

## Example

```toml
[package.metadata.vcpkg]
git = "https://github.com/microsoft/vcpkg"
rev = "4c1db68"
dependencies = ["pkg1", "pkg2"]
```

```
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
dependencies = ["sdl2"]

[package.metadata.vcpkg.target]
x86_64-apple-darwin = { dependencies = ["sdl2", "sdl2-gfx" ] }
x86_64-unknown-linux-gnu = { dependencies = ["sdl2", "opencv"] }
x86_64-pc-windows-msvc = { triplet = "x64-windows-static", dependencies = ["sdl2", "zeromq"] }
```

## Development dependencies

Setting the `dev-dependencies` key allows building libraries that are required by binaries in this crate. Only the packages in the `dependencies` key will be installed if `cargo vcpkg` is run on a crate that depends on this crate.

```toml
[package.metadata.vcpkg]
git = "https://github.com/microsoft/vcpkg"
rev = "4c1db68"
dependencies = ["sdl2"]
dev-dependencies = ["sdl2-image"]

[package.metadata.vcpkg.target]
x86_64-apple-darwin = { dev-dependencies = ["sdl2-gfx" ] }
```

## Installation

Install by running

```
cargo install cargo-vcpkg
```

## License

See LICENSE-APACHE, and LICENSE-MIT for details.
