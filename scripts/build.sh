#!/bin/bash
cd ..
cargo build --release
cd scripts
cp ../target/release/together .
cp -R ../static ./static
docker build -t thankrain/together:1.0 .
docker push thankrain/together:1.0
nomad run together.nomad
