<?php
/*
Plugin Name: Force Application Passwords
*/

// Forces application passwords to be available, even if the site is not using HTTPS
add_filter('wp_is_application_passwords_available', '__return_true');
