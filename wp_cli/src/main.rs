use std::fs::read_to_string;

use base64::prelude::*;
use wp_api::{
    UserCreateParamsBuilder, UserDeleteParams, UserUpdateParamsBuilder, WPAuthentication, WPContext,
};
use wp_networking::WPNetworking;

fn main() {
    // A very naive approach just to get things working for now - this whole code will be deleted
    // soon
    let file_contents = read_to_string("test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let site_url = lines[0];
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    let wp_networking = WPNetworking::new(site_url.into(), authentication);

    let user_list_request = wp_networking
        .api_helper
        .list_users_request(WPContext::Edit, &None);
    let user_list = wp_api::parse_list_users_response_with_edit_context(
        &wp_networking.request(user_list_request).unwrap(),
    )
    .unwrap();
    println!("User List: {:?}", user_list);

    let first_user = user_list.first().unwrap();
    let user_retrieve_request = wp_networking
        .api_helper
        .retrieve_user_request(first_user.id, WPContext::Embed);
    println!(
        "{:?}",
        wp_api::parse_retrieve_user_response_with_embed_context(
            &wp_networking.request(user_retrieve_request).unwrap()
        )
    );

    let user_create_params = UserCreateParamsBuilder::default()
        .username("t_username".to_string())
        .email("t_email@foo.com".to_string())
        .password("t_password".to_string())
        .build()
        .unwrap();

    let user_create_request = wp_networking
        .api_helper
        .create_user_request(&user_create_params);
    let user_create_response = wp_networking.request(user_create_request).unwrap();
    let created_user =
        wp_api::parse_retrieve_user_response_with_edit_context(&user_create_response);

    println!(
        "Create user response: {:?}",
        String::from_utf8_lossy(&user_create_response.body)
    );
    println!("Created User: {:?}", created_user);

    let created_user = created_user.unwrap();
    let user_update_params = UserUpdateParamsBuilder::default()
        .email(Some("t_email_updated@foo.com".to_string()))
        .build()
        .unwrap();
    let user_update_request = wp_networking
        .api_helper
        .update_user_request(created_user.id, &user_update_params);
    let user_update_response = wp_networking.request(user_update_request).unwrap();
    let updated_user =
        wp_api::parse_retrieve_user_response_with_edit_context(&user_update_response);

    println!(
        "Update user response: {:?}",
        String::from_utf8_lossy(&user_update_response.body)
    );
    println!("Updated User: {:?}", updated_user);

    let user_delete_params = UserDeleteParams {
        reassign: first_user.id,
    };
    let user_delete_request = wp_networking
        .api_helper
        .delete_user_request(created_user.id, &user_delete_params);
    let user_delete_response = wp_networking.request(user_delete_request).unwrap();
    println!(
        "Delete user response: {:?}",
        String::from_utf8_lossy(&user_delete_response.body)
    );
    println!(
        "Retrieve current user: {:?}",
        wp_api::parse_retrieve_user_response_with_edit_context(
            &wp_networking
                .request(
                    wp_networking
                        .api_helper
                        .retrieve_current_user_request(WPContext::Edit)
                )
                .unwrap()
        )
    );

    let update_current_user_params = UserUpdateParamsBuilder::default()
        .description(Some("updated_description".to_string()))
        .build()
        .unwrap();
    let update_current_user_request = wp_networking
        .api_helper
        .update_current_user_request(&update_current_user_params);
    let update_current_user_response = wp_networking.request(update_current_user_request).unwrap();
    let updated_current_user =
        wp_api::parse_retrieve_user_response_with_edit_context(&update_current_user_response);
    println!(
        "Update current user response: {:?}",
        String::from_utf8_lossy(&update_current_user_response.body)
    );
    println!("Updated Current User: {:?}", updated_current_user);

    // Remember to use a temporary user to test this
    // println!(
    //     "Delete current user: {:?}",
    //     String::from_utf8_lossy(
    //         &wp_networking
    //             .request(
    //                 wp_networking
    //                     .api_helper
    //                     .delete_current_user_request(&UserDeleteParams {
    //                         reassign: first_user.id
    //                     })
    //             )
    //             .unwrap()
    //             .body
    //     )
    // );
}
