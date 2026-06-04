<?php
/*
Plugin Name: Register Test Meta Fields for Integration Tests
Description: Registers post meta keys (wp_rs_test_string, wp_rs_test_number) with show_in_rest for the wordpress-rs PostMeta integration tests.
Version: 1.0
*/

add_action('init', function () {
    register_post_meta('post', 'wp_rs_test_string', array(
        'type'         => 'string',
        'single'       => true,
        'show_in_rest' => true,
        'auth_callback' => function () {
            return current_user_can('edit_posts');
        },
    ));

    register_post_meta('post', 'wp_rs_test_number', array(
        'type'         => 'integer',
        'single'       => true,
        'show_in_rest' => true,
        'auth_callback' => function () {
            return current_user_can('edit_posts');
        },
    ));
});
