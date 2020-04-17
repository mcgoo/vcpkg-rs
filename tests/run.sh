#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

export VCPKG_ROOT=$SCRIPTDIR/../vcp

for port in harfbuzz ; do
    # check that the port fails before it is installed
    $VCPKG_ROOT/vcpkg remove $port  || true
    cargo clean --manifest-path $port/Cargo.toml
    cargo run --manifest-path $port/Cargo.toml && exit 2
    echo THIS FAILURE IS EXPECTED
    echo This is to ensure that we are not spuriously succeding because the libraries already exist someone on the build machine.
    $VCPKG_ROOT/vcpkg install $port
    cargo run --manifest-path $port/Cargo.toml
done
