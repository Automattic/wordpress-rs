use base64::prelude::*;
use std::fs::read_to_string;
use wp_api::{UserCreateParamsBuilder, UserId, UserUpdateParamsBuilder, WPAuthentication};

use wp_networking::AsyncWPNetworking;

mod wp_db;

fn wp_networking() -> AsyncWPNetworking {
    let file_contents = read_to_string("../test_credentials").unwrap();
    let lines: Vec<&str> = file_contents.lines().collect();
    let site_url = lines[0];
    let auth_base64_token = BASE64_STANDARD.encode(format!("{}:{}", lines[1], lines[2]));

    let authentication = WPAuthentication::AuthorizationHeader {
        token: auth_base64_token,
    };

    AsyncWPNetworking::new(site_url.into(), authentication)
}

// async fn list_users_with_edit_context() -> Vec<UserWithEditContext> {
//     let user_list_request = wp_networking()
//         .api_helper
//         .list_users_request(WPContext::Edit, &None);
//     wp_api::parse_list_users_response_with_edit_context(
//         &wp_networking()
//             .async_request(user_list_request)
//             .await
//             .unwrap(),
//     )
//     .unwrap()
// }

// #[tokio::test]
// async fn test_list_users() {
//     let users_from_db = fetch_db_users().await.unwrap();
//     let users_from_api = list_users_with_edit_context().await;
//     users_from_db
//         .iter()
//         .zip(users_from_api.iter())
//         .for_each(|(db_user, api_user)| {
//             assert_eq!(wp_api::UserId(db_user.id as i32), api_user.id);
//             assert_eq!(db_user.username, api_user.username);
//             assert_eq!(db_user.slug, api_user.slug);
//             assert_eq!(db_user.email, api_user.email);
//             assert_eq!(db_user.url, api_user.url);
//             assert_eq!(
//                 db_user.registered_date,
//                 api_user
//                     .registered_date
//                     .parse::<chrono::DateTime<chrono::Utc>>()
//                     .unwrap()
//             );
//             assert_eq!(db_user.name, api_user.name);
//         });
// }

#[tokio::test]
async fn create_test_user() {
    wp_db::run_and_restore(|mut db| async move {
        let username = "t_username";
        let email = "t_email@foo.com";

        // Create a user using the API
        let user_create_params = UserCreateParamsBuilder::default()
            .username(username.to_string())
            .email(email.to_string())
            .password("t_password".to_string())
            .build()
            .unwrap();
        let user_create_request = wp_networking()
            .api_helper
            .create_user_request(&user_create_params);
        let user_create_response = wp_networking().async_request(user_create_request).await;
        assert!(user_create_response.is_ok());
        let created_user =
            wp_api::parse_retrieve_user_response_with_edit_context(&user_create_response.unwrap())
                .unwrap();

        // Assert that the user is in DB
        let created_user_from_db = db.fetch_db_user(created_user.id.0 as u64).await.unwrap();
        assert_eq!(created_user_from_db.username, username);
        assert_eq!(created_user_from_db.email, email);
    })
    .await;
}

#[tokio::test]
async fn test_update_user() {
    wp_db::run_and_restore(|mut db| async move {
        let new_slug = "new_slug";

        // Find the id of the first user from DB
        let users_from_db = db.fetch_db_users().await.unwrap();
        let first_user = users_from_db.first().unwrap();
        let first_user_id = UserId(first_user.id as i32);

        // Update the user's slug using the API and ensure it's successful
        let user_update_params = UserUpdateParamsBuilder::default()
            .slug(Some(new_slug.to_string()))
            .build()
            .unwrap();
        let user_update_request = wp_networking()
            .api_helper
            .update_user_request(first_user_id, &user_update_params);
        let user_update_response = wp_networking().async_request(user_update_request).await;
        assert!(user_update_response.is_ok());

        // Assert that the DB record of the user is updated with the new slug
        let first_user_after_update = db.fetch_db_user(first_user.id).await.unwrap();
        assert_eq!(first_user_after_update.slug, new_slug);
    })
    .await;
}

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
// }
