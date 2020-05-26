#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

pushd ../cargo-vcpkg
cargo install --path .
popd

cargo vcpkg install --manifest-path=wstest/Cargo.toml
cargo run --manifest-path=wstest/Cargo.toml
