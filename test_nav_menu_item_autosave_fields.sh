#!/bin/bash

# Script to test all nav menu item autosave fields
# This script tests each field individually to see if it gets saved in the autosave

# Don't exit on errors - we want to test all fields even if some fail
set +e

CREDS_FILE="test_credentials.json"
ADMIN_USER=$(jq -r '.admin_username' "$CREDS_FILE")
ADMIN_PASS=$(jq -r '.admin_password' "$CREDS_FILE")
MENU_ITEM_ID=$(jq -r '.nav_menu_item_id' "$CREDS_FILE")
BASE_URL="http://localhost/wp-json/wp/v2"

echo "========================================="
echo "Testing Nav Menu Item Autosave Fields"
echo "========================================="
echo "Menu Item ID: $MENU_ITEM_ID"
echo ""

# Function to restore server
restore_server() {
    echo "Restoring server..."
    make -C .. restore-server > /dev/null 2>&1
    sleep 2
    echo "Server restored"
    echo ""
}

# Function to test a field
test_field() {
    local field_name=$1
    local field_value=$2
    local jq_path=$3
    local expected_value=$4

    echo "========================================="
    echo "Testing field: $field_name"
    echo "========================================="

    # Create autosave with the field
    echo "Creating autosave with $field_name..."
    RESPONSE=$(curl --silent --user "$ADMIN_USER:$ADMIN_PASS" \
        -H "Content-Type: application/json" \
        -d "$field_value" \
        "$BASE_URL/menu-items/$MENU_ITEM_ID/autosaves")

    AUTOSAVE_ID=$(echo "$RESPONSE" | jq -r '.id')
    echo "Autosave created with ID: $AUTOSAVE_ID"

    # Check if autosave creation failed
    if [ "$AUTOSAVE_ID" = "null" ] || [ -z "$AUTOSAVE_ID" ]; then
        ERROR=$(echo "$RESPONSE" | jq -r '.message // .code')
        echo "❌ ERROR: Failed to create autosave - $ERROR"
        echo ""
        return
    fi

    # Retrieve the autosave
    echo "Retrieving autosave..."
    AUTOSAVE=$(curl --silent --user "$ADMIN_USER:$ADMIN_PASS" \
        "$BASE_URL/menu-items/$MENU_ITEM_ID/autosaves/$AUTOSAVE_ID")

    # Check if field was saved
    FIELD_VALUE=$(echo "$AUTOSAVE" | jq "$jq_path")
    echo "Expected: $expected_value"
    echo "Actual: $FIELD_VALUE"

    # Compare with expected value
    if echo "$FIELD_VALUE" | grep -q "$expected_value"; then
        echo "✅ SUCCESS: Field was saved correctly"
    else
        echo "❌ FAILED: Field was NOT saved or value is incorrect"
    fi

    echo ""
}

# Test 1: title
test_field "title" '{"title":"TEST AUTOSAVE TITLE"}' '.title.rendered' "TEST AUTOSAVE TITLE"
restore_server

# Test 2: type
test_field "type" '{"type":"post_type"}' '.type' "post_type"
restore_server

# Test 3: status
test_field "status" '{"status":"draft"}' '.status' "draft"
restore_server

# Test 4: attr_title
test_field "attr_title" '{"attr_title":"Test Attribute Title"}' '.attr_title' "Test Attribute Title"
restore_server

# Test 5: classes
test_field "classes" '{"classes":["test-class-1","test-class-2"]}' '.classes' "test-class-1"
restore_server

# Test 6: description
test_field "description" '{"description":"Test Description"}' '.description' "Test Description"
restore_server

# Test 7: menu_order
test_field "menu_order" '{"menu_order":5}' '.menu_order' "5"
restore_server

# Test 8: object
test_field "object" '{"object":"page"}' '.object' "page"
restore_server

# Test 9: object_id
test_field "object_id" '{"object_id":999}' '.object_id' "999"
restore_server

# Test 10: target
test_field "target" '{"target":"_blank"}' '.target' "_blank"
restore_server

# Test 11: url
test_field "url" '{"url":"https://test-autosave.example.com"}' '.url' "test-autosave.example.com"
restore_server

# Test 12: xfn
test_field "xfn" '{"xfn":["friend","colleague"]}' '.xfn' "friend"
restore_server

# Test 13: menus
test_field "menus" '{"menus":[1]}' '.menus' "1"
restore_server

# Test 14: meta
test_field "meta" '{"meta":{}}' '.meta' "{}"
restore_server

echo "========================================="
echo "All tests completed!"
echo "========================================="
