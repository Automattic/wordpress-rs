use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::{
    api_error::{WpApiError, WpErrorCode},
    decimal2::Decimal2,
    wp_com::{
        CouponCode, CurrencyCode, WpComSiteId,
        shopping_cart::{
            CartKey, ShoppingCart, ShoppingCartMessages, ShoppingCartTax, ShoppingCartTaxLocation,
        },
        transactions::{RedeemCartParams, TransactionPayment, TransactionPaymentMethod},
    },
};

/// Builds a cart with nothing in it.
///
/// Redeeming a cart is a real, irreversible purchase that spends the account's
/// credits, so there is no safe happy path to exercise here. An empty cart is
/// rejected with `empty_cart` before the server charges anything, which still
/// covers the URL, authentication and request serialization.
fn empty_cart(site_id: WpComSiteId) -> ShoppingCart {
    ShoppingCart {
        cart_generated_at_timestamp: 0,
        blog_id: site_id,
        cart_key: CartKey::Site { id: site_id },
        coupon: CouponCode(String::new()),
        is_coupon_applied: false,
        has_auto_renew_coupon_been_automatically_applied: false,
        next_domain_is_free: false,
        next_domain_condition: String::new(),
        products: vec![],
        unmerged_products: vec![],
        total_cost: Decimal2::from_hundredths(0),
        currency: CurrencyCode("USD".to_string()),
        total_cost_integer: 0,
        temporary: true,
        tax: ShoppingCartTax {
            display_taxes: false,
            location: ShoppingCartTaxLocation {
                country_code: None,
                postal_code: None,
                subdivision_code: None,
                ip_address: None,
                vat_id: None,
                organization: None,
                address: None,
                city: None,
                is_for_business: None,
            },
        },
        coupon_savings_total_integer: 0,
        sub_total_with_taxes_integer: 0,
        sub_total_integer: 0,
        total_tax: Decimal2::from_hundredths(0),
        total_tax_integer: 0,
        credits: Decimal2::from_hundredths(0),
        credits_integer: 0,
        allowed_payment_methods: vec![],
        is_gift_purchase: false,
        messages: ShoppingCartMessages {
            errors: vec![],
            success: vec![],
            persistent_errors: vec![],
        },
        bundled_domain: None,
    }
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    trials.push(Trial::test("transactions::redeem_empty_cart", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let result = ctx
                    .client
                    .me()
                    .redeem_cart(&RedeemCartParams {
                        cart: empty_cart(ctx.site_id),
                        payment: TransactionPayment {
                            payment_method: TransactionPaymentMethod::UseCredits,
                        },
                        domain_details: None,
                    })
                    .await;

                match result {
                    Ok(_) => Err("expected redeeming an empty cart to fail".into()),
                    Err(WpApiError::WpError { error_code, .. })
                        if error_code == WpErrorCode::CustomError("empty_cart".to_string()) =>
                    {
                        Ok(())
                    }
                    Err(e) => Err(format!("Unexpected error: {e:?}").into()),
                }
            })
        }
    }));

    trials
}
