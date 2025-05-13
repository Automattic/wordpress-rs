#!/bin/bash -eu

# Give read/write permissions to `./` for all users
chmod -R a+rw ./

echo "--- :docker: Setting up Test Server"
make test-server

echo "--- 🧪 Running Swift Tests"
make test-swift-linux
