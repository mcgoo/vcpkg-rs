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

There is a test of using vcpkg to build diesel and it's dependencies that 
should be easy to set up at https://github.com/mcgoo/vcpkg_diesel_build
check out that project and then

```
make_vcpkg_static
build_diesel_static
```
or
```
make_vcpkg_dll
build_diesel_dll
```

and you should get a working binary.
more notes for this work in progress :-
* could run vcpkg and parse it's output to determine what package versions are
installed. at present it just generates plausible link lines
* vcpkg has common include and lib dirs so there is a chance that someone is
going to end up picking up a vcpkg lib on their link line in preference to
some other version at some point. I believe cmake handles this by using
absolute paths for libs wherever possible.
* vcpkg has a per-package output dir that looks like it would be helpful,
but at present it is undocumented and subject to change. (what I read
mentioned the possibility of compressing the contents.)
* could automatically copy dlls to the target directory so that dynamic
builds will actually run
* there is a lib\no_auto_link folder that some packages generate that needs
to be added to the link line
* should it link rust debug builds against debug libraries? it does not at
present.
* there is some (expected) weirdness when you link to a static lib that needs
other components to link against. I was only able to resolve libpq.lib
needing the openssl libs (ssleay32.lib and libssl32.lib) by making the
top level binary crate create a reference using `extern crate openssl-sys

# License
See LICENSE-APACHE, and LICENSE-MIT for details.