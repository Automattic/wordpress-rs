#!/bin/bash -eu
	curl --user test@example.com:"$(jq .admin_password test_credentials.json)" -H "Content-Type: multipart/form-data" -H "Content-Disposition: attachment; filename=sample.jpeg" -X POST --data-binary "@sample.jpeg" http://localhost/wp-json/wp/v2/media/ --verbose
