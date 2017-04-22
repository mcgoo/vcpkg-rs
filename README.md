# vcpkg-rs

[Documentation](https://docs.rs/vcpkg)

This is a helper for finding native MSVC ABI libraries in cargo build scripts. 
It works similarly to https://github.com/alexcrichton/pkg-config-rs
There is a demo repository here https://github.com/mcgoo/vcpkg_diesel_build

# Example

Find the system library named `foo`:

```rust
extern crate vcpkg;

fn main() {
    vcpkg::probe_library("foo").unwrap();
}
```

# License
See LICENSE-APACHE, and LICENSE-MIT for details.