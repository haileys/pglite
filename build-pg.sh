#!/bin/bash
set -exuo pipefail

# start from known directory
cd "$(dirname "$0")"
BASEDIR="$(pwd)"

# use cross compiler
export PATH=$HOME/cross/x86_64-linux-musl/bin:$PATH

# create + enter fresh build dir
rm -rf postgres-build
mkdir -p postgres-build
cd postgres-build

# configure
../postgres/configure \
    --prefix="$BASEDIR/postgres-inst" \
    --host=x86_64-linux-musl \
    --disable-rpath \
    --without-readline \
    --without-zlib \
    --without-jit \
    LDFLAGS=-static \
    CFLAGS="-g3 -gdwarf-4"

# make
make -j 24

# update gen
# ./update-pg-gen.sh
