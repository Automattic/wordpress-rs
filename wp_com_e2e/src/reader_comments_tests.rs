use libtest_mimic::Trial;
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::comments::CommentId;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_comments::{
    ReaderCommentType, ReaderCommentsOrder, ReaderCommentsParams, ReaderCommentsResponse,
    ReaderCommentsStatusFilter, ReaderCommentsTypeFilter,
};
use wp_api::wp_com::reader_likes::ReaderLikesParams;

use crate::context::TestContext;
use crate::reader_likes_tests::{POST, SITE};
use crate::reader_tags_tests::{expect_error_code, signed_out_client};

/// "A New Theme for Short-Form Blogging on WordPress.com", a post on the WordPress.com Blog with
/// 94 comments, most of them replies, and closed to new ones.
const POST_WITH_REPLIES: PostId = PostId(85067);
/// A comment on that post with six likes.
const LIKED_COMMENT: CommentId = CommentId(451572);
/// ma.tt, a public Jetpack site, and a post on it with comments.
const JETPACK_SITE: WpComSiteId = WpComSiteId(1047865);
const JETPACK_POST: PostId = PostId(154723);

async fn comments(
    client: &WpComApiClient,
    site: WpComSiteId,
    post: PostId,
    params: ReaderCommentsParams,
) -> Result<ReaderCommentsResponse, String> {
    let page = client
        .reader_comments()
        .list(&site, &post, &params)
        .await
        .map_err(|e| e.to_string())?
        .data;
    match page.unreadable_comments.first() {
        Some(unreadable) => Err(format!("A comment couldn't be read: {}", unreadable.error)),
        None => Ok(page),
    }
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_comments::list", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page =
                        comments(&ctx.client, SITE, POST, ReaderCommentsParams::default()).await?;
                    if page.comments.len() != 20 || page.found < Some(800) {
                        return Err("Expected the first 20 of over 800 comments".into());
                    }
                    let dates: Vec<_> = page
                        .comments
                        .iter()
                        .map(|comment| comment.date.map(|date| date.0))
                        .collect();
                    if dates.iter().any(Option::is_none) || !dates.is_sorted_by(|a, b| a >= b) {
                        return Err("The comments should be newest first".into());
                    }
                    let comment = &page.comments[0];
                    if comment.post_id != Some(POST)
                        || comment.content.is_none()
                        || comment.author.is_none()
                        || comment.like_count.is_none()
                    {
                        return Err(format!("Not a whole comment: {comment:?}").into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_comments::list_pages", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Signed out, a flat list by offset, oldest first, until a page is empty.
                    let client = signed_out_client(&ctx);
                    let mut ids = Vec::new();
                    let mut found = None;
                    for _ in 0..20 {
                        let params = ReaderCommentsParams {
                            number: Some(100),
                            offset: Some(ids.len() as u32),
                            order: Some(ReaderCommentsOrder::Asc),
                            fields: Some("ID,date,parent".to_string()),
                            ..Default::default()
                        };
                        let page = comments(&client, SITE, POST, params).await?;
                        found = page.found;
                        if page.comments.is_empty() {
                            break;
                        }
                        ids.extend(page.comments.iter().map(|comment| comment.id));
                    }
                    if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
                        return Err("A comment was listed twice".into());
                    }
                    if Some(ids.len() as u64) != found {
                        return Err(
                            format!("The pages have {} of {found:?} comments", ids.len()).into(),
                        );
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_comments::list_hierarchical", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let number = 20;
                    let mut total = 0;
                    let mut found = None;
                    for page_number in 1..=10 {
                        let params = ReaderCommentsParams {
                            number: Some(number),
                            page: Some(page_number),
                            hierarchical: Some(true),
                            ..Default::default()
                        };
                        let page = comments(&client, SITE, POST_WITH_REPLIES, params).await?;
                        found = page.found;
                        total += page.comments.len() as u64;

                        // Every reply comes after the comment it replies to, in the same page.
                        let mut seen = HashSet::new();
                        for comment in &page.comments {
                            if comment
                                .parent_id
                                .is_some_and(|parent| !seen.contains(&parent))
                            {
                                return Err("A reply came before its parent".into());
                            }
                            seen.insert(comment.id);
                        }

                        // `number` counts top-level comments, and a page with fewer is the last.
                        let top_level = page
                            .comments
                            .iter()
                            .filter(|comment| comment.parent_id.is_none())
                            .count();
                        if top_level > number as usize {
                            return Err("A page has too many top-level comments".into());
                        }
                        if top_level < number as usize {
                            break;
                        }
                    }
                    if Some(total) != found {
                        return Err(format!("The pages have {total} of {found:?} comments").into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_comments::list_by_type", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let all =
                        comments(&client, SITE, POST, ReaderCommentsParams::default()).await?;
                    let params = ReaderCommentsParams {
                        comment_type: Some(ReaderCommentsTypeFilter::Pings),
                        ..Default::default()
                    };
                    let pings = comments(&client, SITE, POST, params).await?;
                    if pings.comments.is_empty() || pings.found >= all.found {
                        return Err("Expected some of the comments to be pings".into());
                    }
                    if pings.comments.iter().any(|comment| {
                        !matches!(
                            comment.comment_type,
                            Some(ReaderCommentType::Pingback | ReaderCommentType::Trackback)
                        )
                    }) {
                        return Err("A comment that isn't a ping was returned".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_comments::list_errors", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let list = |post: PostId, params: ReaderCommentsParams| {
                        let client = &client;
                        async move { client.reader_comments().list(&SITE, &post, &params).await }
                    };

                    // A signed-out reader can only ask for approved comments.
                    let params = ReaderCommentsParams {
                        status: Some(ReaderCommentsStatusFilter::All),
                        ..Default::default()
                    };
                    expect_error_code(list(POST, params).await, "unauthorized")?;

                    // 100 comments is the most a page holds.
                    let params = ReaderCommentsParams {
                        number: Some(101),
                        ..Default::default()
                    };
                    expect_error_code(list(POST, params).await, "invalid_number")?;

                    let no_post = PostId(999999999);
                    expect_error_code(
                        list(no_post, ReaderCommentsParams::default()).await,
                        "unknown_post",
                    )
                })
            }
        }),
        Trial::test("reader_comments::list_of_a_jetpack_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let client = signed_out_client(&ctx);
                    let page = |force: Option<&str>| {
                        let params = ReaderCommentsParams {
                            force: force.map(str::to_string),
                            ..Default::default()
                        };
                        let client = &client;
                        async move { comments(client, JETPACK_SITE, JETPACK_POST, params).await }
                    };

                    // WordPress.com's copy of the comments has their likes.
                    let from_wpcom = page(Some("wpcom")).await?;
                    if from_wpcom.comments.is_empty()
                        || from_wpcom
                            .comments
                            .iter()
                            .any(|comment| comment.like_count.is_none() || comment.i_like.is_none())
                    {
                        return Err("With `force=wpcom`, each comment should have its likes".into());
                    }

                    // The site's own comments don't.
                    let from_site = page(None).await?;
                    if from_site
                        .comments
                        .iter()
                        .any(|comment| comment.like_count.is_some())
                    {
                        return Err("Without `force=wpcom`, a comment had likes".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_comments::subscription", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Either answer is fine for a post the account may or may not follow.
                    ctx.client
                        .reader_comments()
                        .subscription(&SITE, &POST)
                        .await
                        .map_err(|e| e.to_string())?;

                    let result = signed_out_client(&ctx)
                        .reader_comments()
                        .subscription(&SITE, &POST)
                        .await;
                    expect_error_code(result, "authorization_required")
                })
            }
        }),
        Trial::test("reader_comments::comment_likes", {
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
