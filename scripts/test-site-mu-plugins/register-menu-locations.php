<?php
/*
Plugin Name: Register Menu Locations for Integration Tests
Description: Registers primary and footer menu locations for integration testing
Version: 1.0
*/

add_action('after_setup_theme', function() {
    register_nav_menus(array(
        'primary' => __('Primary Menu'),
        'footer' => __('Footer Menu'),
    ));
});