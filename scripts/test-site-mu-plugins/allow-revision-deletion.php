<?php
/*
Plugin Name: Allow Revision Deletion for Integration Tests
Description: Allows administrators to delete revisions via REST API for integration testing
Version: 1.0
*/

add_filter('map_meta_cap', function($caps, $cap, $user_id, $args) {
    if ($cap === 'delete_post' && !empty($args[0])) {
        $post = get_post($args[0]);
        if ($post && $post->post_type === 'revision') {
            if (current_user_can('delete_posts')) {
                return ['delete_posts'];
            }
        }
    }
    return $caps;
}, 10, 4);