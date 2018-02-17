#!/bin/bash -x
set -o errexit

cargo web build --release

mkdir -p dist
cp -a target/wasm32-unknown-unknown/release/bui-demo-frontend-yew.* dist/
cp -a static/* dist/
