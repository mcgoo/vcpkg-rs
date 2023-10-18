#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

TRIPLET=wasm32-emscripten

export CARGO_BUILD_TARGET=wasm32-unknown-unknown

export VCPKG_ROOT=$SCRIPTDIR/../vcp

source ../setup_vcp.sh

for port in harfbuzz ; do
    # check that the port fails before it is installed
    $VCPKG_ROOT/vcpkg remove $port:$TRIPLET  || true
    cargo clean --manifest-path $port/Cargo.toml
    cargo build --manifest-path $port/Cargo.toml && exit 2
    echo THIS FAILURE IS EXPECTED
    echo This is to ensure that we are not spuriously succeeding because the libraries already exist somewhere on the build machine.
    $VCPKG_ROOT/vcpkg install $port:$TRIPLET
    cargo build --manifest-path $port/Cargo.toml
done


# check manifest mode

# clean first
cargo clean --manifest-path top-level/Cargo.toml
unset VCPKG_INSTALLED_ROOT
rm -rf $VCPKG_ROOT/installed

cargo build --manifest-path top-level/Cargo.toml && exit 2
echo "This failure is expected, as we haven't installed anything from vcpkg yet."

export VCPKG_INSTALLED_ROOT=$SCRIPTDIR/top-level/vcpkg_installed
pushd top-level
$VCPKG_ROOT/vcpkg install --triplet=$TRIPLET
popd
cargo build --manifest-path top-level/Cargo.toml
unset VCPKG_INSTALLED_ROOT
