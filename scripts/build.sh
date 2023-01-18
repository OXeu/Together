#!/bin/bash
cd ..
cargo build --release
cd scripts
docker build -t thankrain/together:1.0 .
nomad run together.nomad