use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::reader_saved::ReaderSavedPostsParams;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_saved::list_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .reader_saved()
                        .list_posts(&ReaderSavedPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }),
        Trial::test("reader_saved::list_keys", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .reader_saved()
                        .list_keys()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }),
    ]
}
