use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::language::{LanguagesGetParams, WPComLanguage};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    trials.push(Trial::test("languages::get_remote_language_list", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .languages()
                    .get(&LanguagesGetParams::default())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    trials.push(Trial::test("languages::verify_database_sync", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let languages = ctx
                    .client
                    .languages()
                    .get(&LanguagesGetParams::default())
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                // Verify all remote languages have matching local definitions
                for (slug, language) in languages.iter() {
                    let wpcom_language = WPComLanguage::from_slug(slug)
                        .ok_or_else(|| format!("Invalid language slug: {}", slug))?;

                    if wpcom_language.language_id() != language.id {
                        return Err(format!(
                            "Language ID mismatch for {}: expected {}, got {}",
                            slug,
                            language.id,
                            wpcom_language.language_id()
                        ));
                    }

                    if wpcom_language.display_name() != language.name {
                        return Err(format!(
                            "Language name mismatch for {}: expected {}, got {}",
                            slug,
                            language.name,
                            wpcom_language.display_name()
                        ));
                    }
                }

                // Verify all local languages exist in remote
                for language in WPComLanguage::all() {
                    let slug = language.slug();
                    let remote_language = languages
                        .get(&slug)
                        .ok_or_else(|| format!("Remote language not found: {}", slug))?;

                    if remote_language.id != language.language_id() {
                        return Err(format!(
                            "Remote language ID mismatch for {}: expected {}, got {}",
                            slug,
                            language.language_id(),
                            remote_language.id
                        ));
                    }

                    // Verify popularity rank consistency
                    match (language.popular_rank(), remote_language.popularity_rank) {
                        (Some(local), Some(remote)) if local != remote as u8 => {
                            return Err(format!(
                                "Popularity rank mismatch for {}: expected {}, got {}",
                                slug, remote, local
                            ));
                        }
                        (Some(_), None) => {
                            return Err(format!(
                                "Local language {} is popular but remote is not",
                                slug
                            ));
                        }
                        (None, Some(_)) => {
                            return Err(format!(
                                "Remote language {} is popular but local is not",
                                slug
                            ));
                        }
                        _ => {}
                    }
                }

                Ok(())
            })
        }
    }));

    trials
}
