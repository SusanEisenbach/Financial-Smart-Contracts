#!/bin/bash

# Generate test cargo manifest
source ./generate-cargo.sh lib

# Run tests
cargo test --features std $1

# Replace build manifest
source ./generate-cargo.sh cdylib