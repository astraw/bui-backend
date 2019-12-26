#!/bin/bash
set -o errexit

# Install wasm-pack from here https://rustwasm.github.io/wasm-pack/installer/

# Let wasm-pack build everything and put it in `pkg/`.
wasm-pack build --target web

# Copy our static stuff and put it in `pkg/`, too.
cp static/* pkg/

# above built everything, let's now run it locally
echo "Build OK. Now run with:\n"
echo "    microserver --port 8000 --no-spa pkg"
