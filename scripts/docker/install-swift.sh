#!/bin/bash

set -euo pipefail

curl -o swiftly.tar.gz https://download.swift.org/swiftly/linux/swiftly-aarch64.tar.gz
tar zxf swiftly.tar.gz
rm swiftly.tar.gz
./swiftly init --platform=debian12 --assume-yes --skip-install

apt-get -y install libicu-dev libcurl4-openssl-dev libedit-dev libsqlite3-dev libncurses-dev libpython3-dev libxml2-dev uuid-dev git libstdc++-12-dev

swiftly install --use 6.1
