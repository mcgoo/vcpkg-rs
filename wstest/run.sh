#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

unset VCPKG_ROOT
unset RUSTFLAGS
unset VCPKGRS_DYNAMIC

pushd ../cargo-vcpkg
cargo install --path .
popd

cargo vcpkg build --manifest-path=wstest/Cargo.toml
cargo run --manifest-path=wstest/Cargo.toml
