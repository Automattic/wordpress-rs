#!/bin/bash

curl --header "Content-Type: application/json" \
  -u 'test@example.com:xlehGIszv9cA5XgF9rTfCmpY' \
  --request POST \
  --data '{"status":"active", "context": "view"}' \
  "http://localhost/wp-json/wp/v2/plugins/hello-dolly/hello?context=view"
  #--data '{"plugin": "jetpack", "status":"inactive"}' \
