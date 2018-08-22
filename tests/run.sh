#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

export VCPKG_ROOT=$SCRIPTDIR/../vcp

for port in harfbuzz ; do
    $VCPKG_ROOT/vcpkg install $port
    cargo run --manifest-path $port/Cargo.toml
done
