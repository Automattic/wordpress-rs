#!/bin/bash

curl --header "Content-Type: application/json" \
  -u 'test@example.com:ZluvFUYwYOFs5o3u3Th4u3gy' \
  --request POST \
  --data '{"slug":"hello-dolly"}' \
  "http://localhost/wp-json/wp/v2/plugins/"
