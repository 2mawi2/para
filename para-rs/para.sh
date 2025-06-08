#!/bin/sh
# Shim to test Rust implementation
exec "$(pwd)/target/debug/para" "$@"