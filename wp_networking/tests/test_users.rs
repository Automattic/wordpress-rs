use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{UserWithEditContext, WPAuthentication, WPContext};

use wp_networking::WPNetworking;

fn wp_networking() -> WPNetworking {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let site_url = lines[0];
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    WPNetworking::new(site_url.into(), authentication)
}

fn list_users_with_edit_context() -> Vec<UserWithEditContext> {
    let user_list_request = wp_networking()
        .api_helper
        .list_users_request(WPContext::Edit, &None);
    wp_api::parse_list_users_response_with_edit_context(
        &wp_networking().request(user_list_request).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn test_list_users() {
    let users_from_db = fetch_db_users().await.unwrap();
    let users_from_api = tokio::task::spawn_blocking(move || list_users_with_edit_context())
        .await
        .unwrap();
    users_from_db
        .iter()
        .zip(users_from_api.iter())
        .for_each(|(db_user, api_user)| {
            assert_eq!(wp_api::UserId(db_user.user_id as i32), api_user.id);
            assert_eq!(db_user.user_login, api_user.username);
            assert_eq!(db_user.user_nicename, api_user.slug);
            assert_eq!(db_user.user_email, api_user.email);
            assert_eq!(db_user.user_url, api_user.url);
            assert_eq!(
                db_user.user_registered,
                api_user
                    .registered_date
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap()
            );
            assert_eq!(db_user.display_name, api_user.name);
        });
}

use sqlx::{mysql::MySqlConnectOptions, ConnectOptions, MySqlConnection};

#[derive(Debug, sqlx::FromRow)]
pub struct DbUser {
    #[sqlx(rename = "ID")]
    user_id: u64,
    user_login: String,
    user_nicename: String,
    user_email: String,
    user_url: String,
    user_registered: chrono::DateTime<chrono::Utc>,
    display_name: String,
}

async fn fetch_db_users() -> Result<Vec<DbUser>, sqlx::Error> {
    let mut conn = db().await?;
    sqlx::query_as("SELECT * FROM wp_users")
        .fetch_all(&mut conn)
        .await
}

async fn db() -> Result<MySqlConnection, sqlx::Error> {
    let options = MySqlConnectOptions::new()
        .host("localhost")
        .username("wordpress")
        .password("wordpress")
        .database("wordpress");
    MySqlConnectOptions::connect(&options).await
}

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
