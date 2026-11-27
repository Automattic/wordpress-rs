use libtest_mimic::{Failed, Trial};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::auth::WpAuthenticationProvider;
use wp_api::comments::CommentId;
use wp_api::posts::PostId;
use wp_api::prelude::{WpApiClientDelegate, WpApiError, WpErrorCode};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_likes::ReaderLikesParams;

use crate::context::TestContext;

/// The WordPress.com Blog, which is public.
pub(crate) const SITE: WpComSiteId = WpComSiteId(3584907);
/// "Challenge for 2011: Want to blog more often?", a post from 2010 with over 1,700 likes and
/// 800 comments, long closed to new ones.
pub(crate) const POST: PostId = PostId(6014);
/// How many times a trial that reads a whole list walks it, looking for a walk the list didn't
/// change during.
const WALKS: usize = 3;
/// A comment with six likes, on "A New Theme for Short-Form Blogging on WordPress.com", a post
/// on the WordPress.com Blog closed to new comments.
const LIKED_COMMENT: CommentId = CommentId(451572);

/// A client with no token, for what the Reader shows signed out.
fn signed_out_client(ctx: &TestContext) -> WpComApiClient {
    WpComApiClient::new(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::none().into(),
        ..ctx.delegate.clone()
    })
}

/// Passes when the request failed with this WordPress.com error code.
fn expect_error_code<T>(result: Result<T, WpApiError>, code: &str) -> Result<(), Failed> {
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
        Trial::test("reader_likes::post_likes", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = ctx
                        .client
                        .reader_likes()
                        .post_likes(&SITE, &POST, &ReaderLikesParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if let Some(unreadable) = page.unreadable_likes.first() {
                        return Err(format!("A like couldn't be read: {}", unreadable.error).into());
                    }
                    if page.likes.len() != 20 {
                        return Err(format!("Expected 20 likes, got {}", page.likes.len()).into());
                    }
                    if page.found < Some(1000) || page.i_like.is_none() {
                        return Err("The page is missing its count or whether I like it".into());
                    }
                    if page.likes.iter().any(|like| like.date_liked.is_none()) {
                        return Err("A like has no date".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_likes::post_likes_pages", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Signed out, every page by the date of its last like. More than 90 are
                    // asked for, to see that 90 is the most the API returns.
                    //
                    // The pages must list exactly the count the API gives. Someone can like or
                    // unlike the post during a walk, which moves the pages and the count, so a
                    // walk that saw the count change is walked again.
                    let client = signed_out_client(&ctx);
                    let number = 200;
                    for _ in 0..WALKS {
                        let mut params = ReaderLikesParams {
                            number: Some(number),
                            ..Default::default()
                        };
                        let mut users = Vec::new();
                        let mut counts = HashSet::new();
                        for _ in 0..60 {
                            let page = client
                                .reader_likes()
                                .post_likes(&SITE, &POST, &params)
                                .await
                                .map_err(|e| e.to_string())?
                                .data;
                            if page.likes.len() > 90 {
                                return Err("A page has more than 90 likes".into());
                            }
                            users.extend(page.likes.iter().map(|like| like.id));
                            counts.insert(page.found);
                            match page.next_page_params(number) {
                                Some(next) => params = next,
                                None => break,
                            }
                        }
                        let [found] = counts.into_iter().collect::<Vec<_>>()[..] else {
                            // The count changed during the walk.
                            continue;
                        };
                        let distinct: HashSet<_> = users.iter().collect();
                        if distinct.len() != users.len() {
                            return Err("A like was listed twice".into());
                        }
                        if Some(users.len() as u64) != found {
                            return Err(format!(
                                "The pages have {} of {found:?} likes",
                                users.len()
                            )
                            .into());
                        }
                        return Ok(());
                    }
                    Err(
                        format!("The post's like count changed during each of {WALKS} walks")
                            .into(),
                    )
                })
            }
        }),
        Trial::test("reader_likes::post_likes_exclude", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let page = |exclude| {
                        let params = ReaderLikesParams {
                            number: Some(5),
                            exclude,
                            ..Default::default()
                        };
                        let client = &client;
                        async move {
                            client
                                .reader_likes()
                                .post_likes(&SITE, &POST, &params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };

                    let first = page(vec![]).await?;
                    let excluded: Vec<_> = first.likes.iter().take(2).map(|like| like.id).collect();
                    if excluded.len() != 2 {
                        return Err("The post has too few likes".into());
                    }
                    // Both are left out, and the count still includes them.
                    let rest = page(excluded.clone()).await?;
                    if rest.likes.iter().any(|like| excluded.contains(&like.id)) {
                        return Err("An excluded user is on the page".into());
                    }
                    if rest.found != first.found || rest.likes.first() != first.likes.get(2) {
                        return Err("The page isn't the same list without the two users".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_likes::post_likes_of_a_post_that_does_not_exist", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let result = signed_out_client(&ctx)
                        .reader_likes()
                        .post_likes(&SITE, &PostId(999999999), &ReaderLikesParams::default())
                        .await;
                    expect_error_code(result, "unknown_post")
                })
            }
        }),
        Trial::test("reader_likes::comment_likes", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    // The page has every one of the comment's few likes, as many as its count
                    // says. A like or unlike as the page is put together can leave the two apart
                    // for a moment, so the page is asked for again before that fails the trial.
                    let mut answers = Vec::new();
                    let page = loop {
                        let page = client
                            .reader_likes()
                            .comment_likes(&SITE, &LIKED_COMMENT, &ReaderLikesParams::default())
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if let Some(unreadable) = page.unreadable_likes.first() {
                            return Err(
                                format!("A like couldn't be read: {}", unreadable.error).into()
                            );
                        }
                        if page.likes.is_empty() {
                            return Err("The comment should have likes".into());
                        }
                        if page.found == Some(page.likes.len() as u64) {
                            break page;
                        }
                        answers.push((page.likes.len(), page.found));
                        if answers.len() == 3 {
                            return Err(format!(
                                "Each page had a different number of likes from its count, as \
                                 (likes, count): {answers:?}"
                            )
                            .into());
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    };
                    if page.likes.iter().any(|like| like.date_liked.is_none()) {
                        return Err("A like has no date".into());
                    }
                    // A page that isn't full is the last.
                    if page.next_page_params(20).is_some() {
                        return Err("There shouldn't be another page".into());
                    }

                    let result = client
                        .reader_likes()
                        .comment_likes(&SITE, &CommentId(999999999), &ReaderLikesParams::default())
                        .await;
                    expect_error_code(result, "unknown_comment")
                })
            }
        }),
    ]
}
