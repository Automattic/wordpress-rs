#!/bin/bash

set -euo pipefail

curl -s -o swiftly.tar.gz "https://download.swift.org/swiftly/linux/swiftly-$(uname -m).tar.gz"
tar zxf swiftly.tar.gz
rm swiftly.tar.gz

# If you get an "Unsupported Linux platform" error, check out the supported platforms here:
# https://github.com/swiftlang/swiftly/blob/1.1.0/Sources/LinuxPlatform/Linux.swift#L12-L20
./swiftly init --assume-yes --skip-install

apt-get -y -qq install libicu-dev libcurl4-openssl-dev libedit-dev libsqlite3-dev libncurses-dev libpython3-dev libxml2-dev uuid-dev git libstdc++-12-dev

echo "Installing Swift..."
swiftly install --progress-file /dev/null --use 6.1
