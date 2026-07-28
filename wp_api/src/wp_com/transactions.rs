use crate::{
    date::WpDateString,
    decimal2::Decimal2,
    wp_com::{
        CurrencyCode, WpComSiteId,
        domains::CountryCode,
        me::DomainContactInformation,
        products::{ProductId, ProductSlug, ProductType},
        shopping_cart::ShoppingCart,
    },
    wp_content_string_id, wp_content_u64_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_u64_or_string_or_none_as_t;

wp_content_u64_id!(ReceiptId);
wp_content_u64_id!(OrderId);

// Identifies the subscription a purchase created or renewed.
wp_content_u64_id!(OwnershipId);

// Localized name of a tax, e.g. `"VAT"`, `"GST"`.
wp_content_string_id!(TaxName);

// The vendor's registration ID for a given tax.
wp_content_string_id!(TaxVendorId);

/// How a transaction is paid for.
///
/// Only credit redemption is offered, which is all this client needs. The
/// server accepts several other methods (stored cards, Stripe, PayPal and
/// various redirect processors); adding one here is not just another string,
/// because a payment that needs the user to be sent elsewhere answers with a
/// `redirect_url`/`qr_code` body instead of a receipt, and
/// [`TransactionReceipt`] cannot represent that.
#[derive(Debug, Clone, Serialize, uniffi::Enum)]
pub enum TransactionPaymentMethod {
    /// Charge the account's WordPress.com credits.
    #[serde(rename = "WPCOM_Billing_WPCOM")]
    UseCredits,
}

/// Payment details for a transaction.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct TransactionPayment {
    pub payment_method: TransactionPaymentMethod,
}

/// Parameters for `POST /me/transactions`.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct RedeemCartParams {
    /// The cart to redeem, as returned by `POST /me/shopping-cart/<cart_key>`.
    pub cart: ShoppingCart,
    pub payment: TransactionPayment,
    /// WHOIS contact information. Required when the cart contains a domain
    /// registration or transfer and isn't made up purely of renewals; ignored
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub domain_details: Option<DomainContactInformation>,
}

/// Response from `POST /me/transactions` — a receipt for the redeemed cart.
///
/// The API also declares `error_code` and `error_message`, which are omitted
/// here: the backend hardcodes both to an empty string on this path and
/// reports real failures as HTTP errors.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TransactionReceipt {
    pub receipt_id: ReceiptId,
    /// The billing order behind this receipt.
    ///
    /// `None` when the payment processor didn't return one — the server sends
    /// an empty string in that case, after the transaction has already been
    /// charged.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none_as_t")]
    #[uniffi(default = None)]
    pub order_id: Option<OrderId>,
    /// Whether every product in the cart was purchased.
    ///
    /// `false` does not mean nothing happened. Receiving this type at all means
    /// the transaction was charged and a receipt exists; `false` narrows that
    /// to a partial result, where some products couldn't be provisioned and are
    /// listed in [`failed_purchases`](Self::failed_purchases). Requests that
    /// fail outright — an empty cart, missing contact details, insufficient
    /// credits — return an HTTP error rather than a receipt.
    pub success: bool,
    /// Completed purchases, keyed by the site they belong to.
    pub purchases: HashMap<WpComSiteId, Vec<TransactionPurchase>>,
    /// Products that could not be purchased, keyed by site. Empty when
    /// everything succeeded.
    pub failed_purchases: HashMap<WpComSiteId, Vec<TransactionFailedPurchase>>,
    /// Receipt total formatted for display (e.g. `"C$33.90"`).
    pub display_price: String,
    /// Receipt total in the smallest unit of [`currency`](Self::currency).
    pub price_integer: u64,
    /// Receipt total as a decimal amount.
    pub price_float: Decimal2,
    pub currency: CurrencyCode,
    pub is_gift_purchase: bool,
    /// Whether the receipt is for a Gravatar domain purchase.
    pub is_gravatar_domain: bool,
}

/// A product successfully purchased as part of a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TransactionPurchase {
    pub product_id: ProductId,
    pub product_slug: ProductSlug,
    pub product_name: String,
    /// Abbreviated product name, when the product defines one.
    pub product_name_short: Option<String>,
    pub product_type: ProductType,
    pub is_domain_registration: bool,
    /// The domain name, for domain products. Products of other types either
    /// store their own identifier here or leave it unset.
    pub meta: Option<String>,
    /// The site this purchase belongs to.
    pub blog_id: Option<WpComSiteId>,
    /// The subscription this purchase created or renewed.
    pub ownership_id: Option<OwnershipId>,
    pub is_renewal: bool,
    pub will_auto_renew: bool,
    pub is_gift_purchase: bool,
    pub free_trial: bool,
    /// Email address of the buyer.
    pub user_email: String,
    /// Amount paid in the smallest unit of the receipt's currency.
    pub price_integer: u64,
    /// When the purchased subscription expires. `None` for products that
    /// don't create a subscription.
    #[serde(default)]
    #[uniffi(default = None)]
    pub expiry: Option<WpDateString>,
    /// Details of the entity that collected tax on this purchase, present
    /// only when tax was charged.
    #[serde(default)]
    #[uniffi(default = None)]
    pub tax_vendor_info: Option<TransactionTaxVendorInfo>,
    /// Whether provisioning is deferred, for domain transfers.
    #[serde(default)]
    #[uniffi(default = false)]
    pub delayed_provisioning: bool,
    /// Whether this is a 100-year domain registration or transfer.
    #[serde(default)]
    #[uniffi(default = false)]
    pub is_hundred_year_domain: bool,
    /// For a mapped subdomain, whether its root domain is registered with
    /// WordPress.com.
    #[serde(default)]
    #[uniffi(default = false)]
    pub is_root_domain_with_us: bool,
    /// The purchased quantity, for products sold in upgradable quantities.
    #[serde(default)]
    #[uniffi(default = None)]
    pub new_quantity: Option<u32>,
    /// Where to send the user to finish setting up a marketplace SaaS product.
    #[serde(default)]
    #[uniffi(default = None)]
    pub saas_redirect_url: Option<String>,
}

/// A product that could not be purchased as part of a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TransactionFailedPurchase {
    pub product_id: ProductId,
    pub product_name: String,
    pub product_slug: ProductSlug,
    /// The domain name, for domain products. Products of other types either
    /// store their own identifier here or leave it unset.
    pub product_meta: Option<String>,
    pub product_cost: Decimal2,
}

/// The entity that collected tax on a purchase, for display on a receipt.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TransactionTaxVendorInfo {
    pub country_code: CountryCode,
    /// The entity's mailing address, one entry per line.
    pub address: Vec<String>,
    /// Each tax charged, mapped to the vendor's registration ID for it.
    pub tax_name_and_vendor_id_array: HashMap<TaxName, TaxVendorId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn receipt(json_file_path: &str) -> TransactionReceipt {
        let file = File::open(json_file_path).expect("Failed to open file");
        serde_json::from_reader(file).expect("Unable to parse JSON")
    }

    #[test]
    fn test_redeem_cart_success_deserialization() {
        let receipt = receipt("tests/wpcom/transactions/redeem-cart-success.json");

        assert_eq!(receipt.receipt_id, ReceiptId(11223344));
        assert_eq!(receipt.order_id, Some(OrderId(55667788)));
        assert!(receipt.success);
        assert!(receipt.failed_purchases.is_empty());
        assert_eq!(receipt.price_integer, 2825);
        assert_eq!(receipt.price_float, Decimal2::from_hundredths(2825));
        assert_eq!(receipt.currency, CurrencyCode("USD".to_string()));

        // The `purchases` map is keyed by site ID, which arrives as a JSON
        // object key — i.e. a string — rather than a number.
        let site_purchases = receipt
            .purchases
            .get(&WpComSiteId(98765432))
            .expect("purchases should be keyed by site ID");
        assert_eq!(site_purchases.len(), 2);

        let domain = &site_purchases[0];
        assert_eq!(domain.product_id, ProductId(501));
        assert_eq!(
            domain.product_slug,
            ProductSlug("dotblog_domain".to_string())
        );
        assert_eq!(domain.product_type, ProductType("domain_reg".to_string()));
        assert!(domain.is_domain_registration);
        assert_eq!(domain.meta.as_deref(), Some("fake-test-domain.blog"));
        assert_eq!(domain.ownership_id, Some(OwnershipId(77001)));
        assert_eq!(domain.blog_id, Some(WpComSiteId(98765432)));
        assert_eq!(
            domain.expiry,
            Some(WpDateString("2030-01-15".to_string())),
            "expiry is a date without a time component"
        );
        assert!(domain.product_name_short.is_none());
        assert!(domain.will_auto_renew);
        assert!(!domain.is_renewal);

        // Optional fields absent from a plain domain purchase.
        assert!(!domain.delayed_provisioning);
        assert!(!domain.is_hundred_year_domain);
        assert!(!domain.is_root_domain_with_us);
        assert!(domain.new_quantity.is_none());
        assert!(domain.saas_redirect_url.is_none());

        let tax = domain
            .tax_vendor_info
            .as_ref()
            .expect("a taxed purchase should have tax_vendor_info");
        assert_eq!(tax.country_code, CountryCode("CA".to_string()));
        assert_eq!(tax.address.len(), 4);
        assert_eq!(
            tax.tax_name_and_vendor_id_array
                .get(&TaxName("GST".to_string())),
            Some(&TaxVendorId("FAKE-GST-000123".to_string()))
        );
    }

    /// There is no way to capture a real partial failure without paying for
    /// one, so this fixture is hand-written from the
    /// `WPCOM_Store_API::checkout()` assembly logic. Its purchases follow that
    /// logic's constraints — the fields gated on a subscription
    /// (`expiry`, `ownership_id`, `is_renewal`, `will_auto_renew`,
    /// `new_quantity`, `saas_redirect_url`) only appear together — and it
    /// covers the fields that show up solely for transfers, marketplace
    /// products and quantity upgrades.
    #[test]
    fn test_redeem_cart_partial_failure_deserialization() {
        let receipt = receipt("tests/wpcom/transactions/redeem-cart-partial-failure.json");

        assert!(
            !receipt.success,
            "a partial failure is signalled through success and failed_purchases"
        );

        let failed = receipt
            .failed_purchases
            .get(&WpComSiteId(12345678))
            .expect("failed_purchases should be keyed by site ID");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].product_id, ProductId(603));
        assert_eq!(
            failed[0].product_meta.as_deref(),
            Some("fake-failed-domain.example")
        );
        assert_eq!(failed[0].product_cost, Decimal2::from_hundredths(1800));

        let purchases = receipt
            .purchases
            .get(&WpComSiteId(12345678))
            .expect("purchases should be keyed by site ID");

        let transfer = &purchases[0];
        assert!(transfer.delayed_provisioning);
        assert!(transfer.is_hundred_year_domain);
        assert!(transfer.is_root_domain_with_us);
        assert_eq!(transfer.product_name_short.as_deref(), Some("Transfer"));

        let saas = &purchases[1];
        assert_eq!(saas.new_quantity, Some(25));
        assert_eq!(
            saas.saas_redirect_url.as_deref(),
            Some("https://example.com/setup?intent-id=fake-intent")
        );
        assert!(saas.meta.is_none());

        // A product that creates no subscription: no expiry, no ownership, and
        // nothing to renew.
        let one_time = &purchases[2];
        assert!(one_time.expiry.is_none());
        assert!(one_time.ownership_id.is_none());
        assert!(!one_time.will_auto_renew);
        assert!(!one_time.is_renewal);
        assert!(one_time.product_name_short.is_none());

        let tax = saas
            .tax_vendor_info
            .as_ref()
            .expect("expected tax_vendor_info");
        assert_eq!(tax.country_code, CountryCode("IE".to_string()));
        assert_eq!(
            tax.tax_name_and_vendor_id_array
                .get(&TaxName("VAT".to_string())),
            Some(&TaxVendorId("FAKE-VAT-000456".to_string()))
        );
    }

    #[test]
    fn test_receipt_with_empty_order_id() {
        // The server defaults `order_id` to an empty string and only fills it
        // in when the payment processor returned one. The receipt still has to
        // deserialize — by this point the transaction has been charged.
        let mut json: serde_json::Value = serde_json::from_reader(
            File::open("tests/wpcom/transactions/redeem-cart-success.json")
                .expect("Failed to open file"),
        )
        .expect("Unable to parse JSON");
        json["order_id"] = serde_json::json!("");

        let receipt: TransactionReceipt =
            serde_json::from_value(json).expect("a receipt without an order id should deserialize");

        assert!(receipt.order_id.is_none());
        assert_eq!(receipt.receipt_id, ReceiptId(11223344));
    }

    fn fixture_cart() -> ShoppingCart {
        let file = File::open("tests/wpcom/shopping_cart/cart-with-site.json")
            .expect("Failed to open file");
        serde_json::from_reader(file).expect("Unable to parse JSON")
    }

    #[test]
    fn test_redeem_cart_params_serializes_credits_payment_method() {
        let params = RedeemCartParams {
            cart: fixture_cart(),
            payment: TransactionPayment {
                payment_method: TransactionPaymentMethod::UseCredits,
            },
            domain_details: None,
        };

        let json: serde_json::Value =
            serde_json::to_value(&params).expect("params should serialize");

        assert_eq!(json["payment"]["payment_method"], "WPCOM_Billing_WPCOM");
        assert!(
            json.get("domain_details").is_none(),
            "domain_details should be omitted rather than sent as null"
        );
        assert_eq!(json["cart"]["cart_key"], 12345678);
    }

    #[test]
    fn test_redeem_cart_params_serializes_domain_details() {
        let params = RedeemCartParams {
            cart: fixture_cart(),
            payment: TransactionPayment {
                payment_method: TransactionPaymentMethod::UseCredits,
            },
            domain_details: Some(DomainContactInformation {
                first_name: Some("Jane".to_string()),
                last_name: Some("Smith".to_string()),
                organization: None,
                address_1: None,
                address_2: None,
                postal_code: None,
                city: None,
                state: None,
                country_code: Some(CountryCode("US".to_string())),
                email: Some("jane@example.com".to_string()),
                phone: None,
                fax: None,
                extra: None,
            }),
        };

        let json: serde_json::Value =
            serde_json::to_value(&params).expect("params should serialize");

        assert_eq!(json["domain_details"]["first_name"], "Jane");
        assert_eq!(json["domain_details"]["country_code"], "US");
        assert_eq!(json["payment"]["payment_method"], "WPCOM_Billing_WPCOM");
    }
}
