use libtest_mimic::{Failed, Trial};
use std::sync::Arc;
use wp_api::prelude::*;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_api::wp_com::reader_tags::ReaderMenuItemKind;

use crate::context::TestContext;

/// A client with no token, for what the Reader shows signed out.
pub(crate) fn signed_out_client(ctx: &TestContext) -> WpComApiClient {
    WpComApiClient::new(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::none().into(),
        ..ctx.delegate.clone()
    })
}

/// Passes when the request failed with this WordPress.com error code.
pub(crate) fn expect_error_code<T>(
    result: Result<T, WpApiError>,
    code: &str,
) -> Result<(), Failed> {
    match result {
        Err(WpApiError::WpError { error_code, .. })
            if error_code == WpErrorCode::CustomError(code.to_string()) =>
        {
            Ok(())
        }
        Err(error) => Err(format!("Expected the error `{code}`, got {error}").into()),
        Ok(_) => Err(format!("Expected the error `{code}`, but the request succeeded").into()),
    }
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_tags::menu", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let menu = ctx
                        .client
                        .reader_tags()
                        .menu()
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if let Some(unreadable) = menu.unreadable_items.first() {
                        return Err(format!("An entry couldn't be read: {unreadable:?}").into());
                    }
                    // Signed in, the menu has more streams than Discover.
                    for key in ["following", "discover", "liked"] {
                        if !menu.default_items.iter().any(|item| item.key == key) {
                            return Err(format!("The menu has no `{key}` stream").into());
                        }
                    }
                    let tags = menu.subscribed_items.iter().chain(&menu.recommended_items);
                    for tag in tags {
                        if tag.kind != ReaderMenuItemKind::Tag
                            || tag.slug.is_none()
                            || tag.tag_id.is_none()
                        {
                            return Err(format!("Not a whole tag: {tag:?}").into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_tags::menu_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let menu = signed_out_client(&ctx)
                        .reader_tags()
                        .menu()
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let streams: Vec<_> = menu
                        .default_items
                        .iter()
                        .map(|item| item.key.as_str())
                        .collect();
                    if streams != ["discover"] {
                        return Err(format!("Expected Discover alone, got {streams:?}").into());
                    }
                    if !menu.subscribed_items.is_empty() {
                        return Err("A signed-out reader follows no tags".into());
                    }
                    if menu.recommended_items.is_empty() {
                        return Err("A signed-out reader should get tags to suggest".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_tags::tag", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let tag = |slug: &str| {
                        let slug = ReaderTagSlug(slug.to_string());
                        let client = &client;
                        async move {
                            client
                                .reader_tags()
                                .tag(&slug)
                                .await
                                .map(|response| response.data.tag)
                                .map_err(|e| e.to_string())
                        }
                    };

                    let photography = tag("photography").await?;
                    if photography.slug.0 != "photography"
                        || photography.title.as_deref() != Some("Photography")
                        || photography.id.is_none()
                    {
                        return Err(format!("Not the tag asked for: {photography:?}").into());
                    }

                    // The slug comes back in lower case.
                    if tag("Photography").await?.slug != photography.slug {
                        return Err("A slug in another case should be the same tag".into());
                    }

                    // A slug that isn't ASCII comes back as it was sent, not percent-encoded.
                    let japanese = tag("写真").await?;
                    if japanese.slug.0 != "写真" {
                        return Err(format!("The slug wasn't decoded: {japanese:?}").into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_tags::tag_that_does_not_exist", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let slug = ReaderTagSlug("wprs-no-such-tag-9431".to_string());
                    let result = ctx.client.reader_tags().tag(&slug).await;
                    expect_error_code(result, "invalid_tag")
                })
            }
        }),
        Trial::test("reader_tags::interests", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let interests = signed_out_client(&ctx)
                        .reader_tags()
                        .interests()
                        .await
                        .map_err(|e| e.to_string())?
                        .data
                        .interests;
                    if interests.is_empty() {
                        return Err("There are no interests to pick from".into());
                    }
                    if interests
                        .iter()
                        .any(|interest| interest.title.is_none() || interest.slug.0.is_empty())
                    {
                        return Err("An interest is missing its title or slug".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}
