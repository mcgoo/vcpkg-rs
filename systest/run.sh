#!/bin/bash
set -ex

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPTDIR

export VCPKG_ROOT=$SCRIPTDIR/../vcp

$VCPKG_ROOT/vcpkg install curl zeromq openssl
cargo run
