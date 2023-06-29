#!/usr/bin/env bash
set -e
set -x
export INSTA_UPDATE=no
pushd borsh
cargo test --no-run
cargo test
cargo test --no-default-features
cargo test --no-default-features --features schema
cargo test --no-default-features --features hashbrown,rc
cargo test --features rc
popd
cargo test --workspace
