#!/bin/bash

set -e

## Create Database Backup (if needed)
backup=.wordpress/wp-content/dump.sql
if [ ! -e "$backup" ]; then
	docker exec -it wordpress-rs-database-1 mariadb-dump -u wordpress -pwordpress --no-tablespaces wordpress > $backup
fi 

export WP_CONTENT_PATH=$(PWD)/.wordpress/wp-content 
export DB_HOSTNAME=localhost 
export API_URL=http://localhost

## Create Plugin Backup
cp -R .wordpress/wp-content/plugins .wordpress/wp-content/plugins-backup

## Run the integration tests
cargo test --no-fail-fast
