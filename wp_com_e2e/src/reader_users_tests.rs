use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_sites::ReaderSiteParams;
use wp_api::wp_com::reader_users::{ReaderUserSuggestionsParams, WpComUserIdentifier};
use wp_api::wp_com::sites::WpComSiteIdentifier;

use crate::context::TestContext;
use crate::reader_post_tests::{expect_error_code, signed_out_client};

/// The WordPress.com Blog, which is public. Its owner is the user the trials look up.
const SITE: WpComSiteId = WpComSiteId(3584907);

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_users::profile", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let site = ctx
                        .client
                        .reader_sites()
                        .site(
                            &WpComSiteIdentifier::Id { value: SITE.0 },
                            &ReaderSiteParams::default(),
                        )
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let owner = site.owner.ok_or("The site should have an owner")?;
                    let (Some(id), Some(login)) = (owner.id, owner.login) else {
                        return Err("The owner should have an ID and a login".into());
                    };

                    // By login, in any case, and by ID, it's the same user.
                    for user in [
                        WpComUserIdentifier::Login {
                            value: login.clone(),
                        },
                        WpComUserIdentifier::Login {
                            value: login.to_uppercase(),
                        },
                        WpComUserIdentifier::Id { value: id },
                    ] {
                        let profile = ctx
                            .client
                            .reader_users()
                            .profile(&user)
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if profile.id != id || profile.login.as_deref() != Some(login.as_str()) {
                            return Err(format!("{user:?} should find the site's owner").into());
                        }
                        if profile.display_name.is_none() || profile.avatar_url.is_none() {
                            return Err("The profile is missing its name or avatar".into());
                        }
                        if profile.primary_site.is_none() {
                            return Err("The owner of a site should have a main site".into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_users::profile_by_an_id_that_is_also_a_login", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // The API reads a number as a login first, and a user's login is `5`.
                    let asked_for = WpComUserId(5);
                    let profile = ctx
                        .client
                        .reader_users()
                        .profile(&WpComUserIdentifier::Id { value: asked_for })
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if profile.login.as_deref() != Some("5") || profile.id == asked_for {
                        return Err("The ID should find the user whose login is `5`".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_users::profile_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing = ctx
                        .client
                        .reader_users()
                        .profile(&WpComUserIdentifier::Login {
                            value: "no-such-user-xyzzy-98765".to_string(),
                        })
                        .await;
                    expect_error_code(missing, "unknown_user")
                })
            }
        }),
        Trial::test("reader_users::profile_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let signed_out = signed_out_client(&ctx)
                        .reader_users()
                        .profile(&WpComUserIdentifier::Id {
                            value: WpComUserId(5),
                        })
                        .await;
                    expect_error_code(signed_out, "authorization_required")
                })
            }
        }),
        Trial::test("reader_users::suggestions", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let suggest = |filter: Option<&str>, image_size| {
                        let params = ReaderUserSuggestionsParams {
                            site_id: Some(ctx.site_id),
                            filter: filter.map(str::to_string),
                            image_size,
                            ..Default::default()
                        };
                        let ctx = &ctx;
                        async move {
                            ctx.client
                                .reader_users()
                                .suggestions(&params)
                                .await
                                .map_err(|e| e.to_string())
                                .map(|response| response.data)
                        }
                    };
                    // The server only suggests the people of a site the account is a member of,
                    // so the trial asks about the account's own test site, which has at least the
                    // account.
                    let all = suggest(None, None).await?;
                    if all.suggestions.is_empty() || !all.unreadable_suggestions.is_empty() {
                        return Err(format!(
                            "The account's own site should have people to suggest, all readable: \
                             {} and {} unreadable",
                            all.suggestions.len(),
                            all.unreadable_suggestions.len()
                        )
                        .into());
                    }
                    if all.suggestions.iter().any(|suggestion| {
                        suggestion.user_login.is_empty() || suggestion.image_url.is_none()
                    }) {
                        return Err("Each suggestion should have a login and an avatar".into());
                    }

                    // A filter by one of their logins keeps that person, and a filter that
                    // matches no one leaves no one.
                    let login = &all.suggestions[all.suggestions.len() - 1].user_login;
                    let filtered = suggest(Some(login), Some(40)).await?;
                    if !filtered
                        .suggestions
                        .iter()
                        .any(|suggestion| &suggestion.user_login == login)
                        || filtered.suggestions.len() > all.suggestions.len()
                    {
                        return Err("A filter by a login should keep that person".into());
                    }
                    if !suggest(Some("no-such-user-xyzzy-98765"), None)
                        .await?
                        .suggestions
                        .is_empty()
                    {
                        return Err("A filter that matches no one should leave no one".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_users::suggestions_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Signed out the API answers, with no one.
                    let signed_out = signed_out_client(&ctx)
                        .reader_users()
                        .suggestions(&ReaderUserSuggestionsParams {
                            site_id: Some(SITE),
                            ..Default::default()
                        })
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if !signed_out.suggestions.is_empty() {
                        return Err("Signed out there should be no one to suggest".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_users::suggestions_without_a_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing = ctx
                        .client
                        .reader_users()
                        .suggestions(&ReaderUserSuggestionsParams::default())
                        .await;
                    expect_error_code(missing, "invalid_query")
                })
            }
        }),
    ]
}
