// use base64::prelude::*;
// use std::fs::read_to_string;
// use wp_api::{
//     UserCreateParamsBuilder, UserUpdateParamsBuilder, UserWithEditContext, WPApiError,
//     WPAuthentication, WPContext, WPNetworkResponse,
// };
//
// use wp_networking::WPNetworking;
//
// fn wp_networking() -> WPNetworking {
//     let file_contents = read_to_string("../test_credentials").unwrap();
//     let lines: Vec<&str> = file_contents.lines().collect();
//     let site_url = lines[0];
//     let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));
//
//     let authentication = WPAuthentication::AuthorizationHeader {
//         token: auth_base64_token,
//     };
//
//     WPNetworking::new(site_url.into(), authentication)
// }
//
// fn list_users_with_edit_context() -> Vec<UserWithEditContext> {
//     let user_list_request = wp_networking()
//         .api_helper
//         .list_users_request(WPContext::Edit, None);
//     wp_api::parse_list_users_response_with_edit_context(
//         &wp_networking().request(user_list_request).unwrap(),
//     )
//     .unwrap()
// }
//
// fn create_test_user() -> (
//     WPNetworkResponse,
//     Result<Option<UserWithEditContext>, WPApiError>,
// ) {
//     let user_create_params = UserCreateParamsBuilder::default()
//         .username("t_username".to_string())
//         .email("t_email@foo.com".to_string())
//         .password("t_password".to_string())
//         .build()
//         .unwrap();
//
//     let user_create_request = wp_networking()
//         .api_helper
//         .create_user_request(user_create_params);
//     let user_create_response = wp_networking().request(user_create_request).unwrap();
//     let created_user =
//         wp_api::parse_retrieve_user_response_with_edit_context(&user_create_response);
//     (user_create_response, created_user)
// }
//
// #[test]
// fn test_list_users() {
//     println!("User List: {:?}", list_users_with_edit_context());
// }
//
// #[test]
// fn test_retrieve_user() {
//     let user_list = list_users_with_edit_context();
//     let user_retrieve_request = wp_networking()
//         .api_helper
//         .retrieve_user_request(user_list.first().unwrap().id, WPContext::Embed);
//     println!(
//         "Retrieve User: {:?}",
//         wp_api::parse_retrieve_user_response_with_embed_context(
//             &wp_networking().request(user_retrieve_request).unwrap()
//         )
//     );
// }
//
// #[test]
// fn test_create_user() {
//     let (user_create_response, created_user) = create_test_user();
//
//     println!(
//         "Create user response: {:?}",
//         std::str::from_utf8(&user_create_response.body)
//     );
//     println!("Created User: {:?}", created_user);
// }
//
// #[test]
// fn test_update_user() {
//     let (_, created_user) = create_test_user();
//     let user_update_params = UserUpdateParamsBuilder::default()
//         .email(Some("t_email_updated@foo.com".to_string()))
//         .build()
//         .unwrap();
//     let user_update_request = wp_networking()
//         .api_helper
//         .update_user_request(created_user.unwrap().unwrap().id, user_update_params);
//     let user_update_response = wp_networking().request(user_update_request).unwrap();
//     let updated_user =
//         wp_api::parse_retrieve_user_response_with_edit_context(&user_update_response);
//
//     println!(
//         "Update user response: {:?}",
//         std::str::from_utf8(&user_update_response.body)
//     );
//     println!("Updated User: {:?}", updated_user);
// }

use sqlx::{mysql::MySqlConnectOptions, ConnectOptions, MySqlConnection};

#[tokio::test]
async fn fetch_db_users() {
    let mut conn = db().await;
    let rows = sqlx::query("SELECT * FROM wp_users")
        .fetch_all(&mut conn)
        .await;
    println!("{:?}", rows);
    println!("{:?}", rows.unwrap().len());
}

async fn db() -> MySqlConnection {
    let options = MySqlConnectOptions::new()
        .host("localhost")
        .username("wordpress")
        .password("wordpress")
        .database("wordpress");
    MySqlConnectOptions::connect(&options).await.unwrap()
}
