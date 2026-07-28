use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::{
    language::WPComLanguage,
    products::{ProductTypeFilter, ProductsParams},
};

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    trials.push(Trial::test("products::list_all", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let products = ctx
                    .client
                    .products()
                    .list(&ProductsParams::default())
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if products.is_empty() {
                    return Err("expected non-empty products list".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("products::list_domains", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let products = ctx
                    .client
                    .products()
                    .list(&ProductsParams {
                        product_type: Some(ProductTypeFilter::Domains),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if products.is_empty() {
                    return Err("expected non-empty domain products list".into());
                }

                // All returned products should be domain-related.
                for (slug, product) in &products {
                    if !product.product_type.0.contains("domain") {
                        return Err(format!(
                            "expected domain product type for {slug}, got {}",
                            product.product_type
                        )
                        .into());
                    }
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("products::list_jetpack", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let products = ctx
                    .client
                    .products()
                    .list(&ProductsParams {
                        product_type: Some(ProductTypeFilter::Jetpack),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if products.is_empty() {
                    return Err("expected non-empty jetpack products list".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("products::list_with_locale", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let products = ctx
                    .client
                    .products()
                    .list(&ProductsParams {
                        locale: Some(WPComLanguage::Spanish),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if products.is_empty() {
                    return Err("expected non-empty products list with locale".into());
                }

                Ok(())
            })
        }
    }));

    trials
}
