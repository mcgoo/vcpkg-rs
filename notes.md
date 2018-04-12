# scratchpad for notes

allow specifying a triple to use using an environment variable. this will
allow setting up a custom "x64-rust-static" triple that dynamically links
to msvcrt, allowing static builds with the default rust.

add information about target triples and target triple selection being
driven by RUSTFLAGS=-Ctarget-feature=+crt-static

add a note that even rust debug builds are linked against the release version
of built libraries

there is a lib\no_auto_link folder that some packages generate that needs
to be added to the link line. this will require finding an example of
a library that uses that feature. (boost?)

vcpkg_cli: make probe failure return a nonzero exit code so the build fails

remove crate doc info about the libname -> package mapping. (why?)

look into the possibility of using dotenv to allow setting VCPKG_ROOT

possibly chase the dependencies
COMMAND powershell -noprofile -executionpolicy Bypass -file ${_VCPKG_TOOLCHAIN_DIR}/msbuild/applocal.ps1
                        -targetBinary $<TARGET_FILE:${name}>
                        -installedDir "${_VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}$<$<CONFIG:Debug>:/debug>/bin"
                        -OutVariable out
)

* could run vcpkg and parse it's output to determine what package versions are
installed.

* could parse vcpkg's installed files list to guess at the names for libraries
and dlls rather than requiring them to be specified.

* could parse vcpkg's installed packages list to determine what other packages
we need to link against.

* vcpkg has common include and lib dirs so there is a chance that someone is
going to end up picking up a vcpkg lib on their link line in preference to
some other version at some point. I believe cmake handles this by using
absolute paths for libs wherever possible. if everything below you in the dependency
tree is looking in vcpkg then everything will agree.

* vcpkg has a per-package output dir that looks like it would be helpful,
but at present it is undocumented and subject to change. (what I read
mentioned the possibility of compressing the contents.)

make it warn if you use something that looks like a vcpkg triple in place of a rust triple

## Making a release

1) run vcpkg_cli and test it
1) run systest\test.cmd
1) check that everything is committed and work dir is clean
1) push to master on gitlab and github
1) check that gitlab ci, appveyor and travis pass
1) update changelog, commit and push
1) update version number in Cargo.toml for the crate to be released
1) commit
1) push
1) wait for the ci to work
1) create a tag for the right crate like `git tag vcpkg-rs-0.2.3`
1) cd to the crate dir and run `run cargo publish`
1) push
