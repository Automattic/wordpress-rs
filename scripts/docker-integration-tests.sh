#!/bin/bash

set -e

apt update
apt install -y default-mysql-client jq

## Create Database Backup (if needed)
backup=/app/.wordpress/wp-content/dump.sql
if [ ! -e "$backup" ]; then
	mariadb-dump -u wordpress -pwordpress --no-tablespaces wordpress -h host.docker.internal > $backup
fi 

## Create Plugin Backup
cp -R /app/.wordpress/wp-content/plugins /app/.wordpress/wp-content/plugins-backup

## Run the integration tests
cargo test --no-fail-fast -p wp_api_integration_tests
