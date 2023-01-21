#!/bin/sh

cargo install cargo-audit cargo-tarpaulin
cargo install --version="~0.6" sqlx-cli --no-default-features --features rustls,postgres