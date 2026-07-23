use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    decimal2::Decimal2,
    wp_com::{
        CurrencyCode, WpComSiteId,
        domains::DomainName,
        me::WpComUserId,
        products::{ProductId, ProductSlug},
        sites::WpComSiteSlug,
    },
};
use serde::{Deserialize, Serialize};

uniffi::custom_newtype!(PurchaseId, u64);
/// Identifies an existing WordPress.com purchase/subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PurchaseId(pub u64);

/// Lifecycle status of a purchase's subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum PurchaseSubscriptionStatus {
    #[serde(rename = "active")]
    Active,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// How a purchase renews or expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum PurchaseExpiryStatus {
    #[serde(rename = "auto-renew")]
    AutoRenew,
    #[serde(rename = "manual-renew")]
    ManualRenew,
    #[serde(rename = "included")]
    Included,
    #[serde(rename = "one-time-purchase")]
    OneTimePurchase,
    #[serde(rename = "expiring")]
    Expiring,
    #[serde(rename = "expired")]
    Expired,
    /// A status not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// Payment method backing a purchase.
///
/// Present only when the purchase is owned by the requesting user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum PaymentType {
    #[serde(rename = "credits")]
    Credits,
    #[serde(rename = "stripe")]
    Stripe,
    #[serde(rename = "paypal")]
    Paypal,
    /// A payment type not covered by the known variants.
    #[serde(untagged)]
    Other(String),
}

/// A downgrade path offered as part of a refund.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct RefundOption {
    /// The product the purchase would be downgraded to.
    pub to_product_id: ProductId,
    /// Refund amount for taking this downgrade.
    pub refund_amount: Decimal2,
    /// Currency symbol for `refund_amount` (e.g. `"$"`).
    pub refund_currency_symbol: String,
}

/// A single purchase (subscription) on a WordPress.com site, as returned by
/// `GET /rest/v1.2/sites/{site_id}/purchases`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePurchase {
    // Identity & product
    #[serde(rename = "ID")]
    pub id: PurchaseId,
    pub user_id: WpComUserId,
    pub blog_id: WpComSiteId,
    pub product_id: ProductId,
    pub product_slug: ProductSlug,
    pub product_name: String,
    /// Coarse product category (e.g. `"bundle"`, `"domain"`).
    pub product_type: String,
    /// Product-specific metadata; for domain purchases this is the domain name.
    pub meta: String,
    /// The bundle purchase this one is attached to, if any.
    #[uniffi(default = None)]
    pub attached_to_purchase_id: Option<PurchaseId>,

    // Site
    pub blogname: String,
    pub domain: DomainName,
    pub site_slug: WpComSiteSlug,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub blog_created_date: Option<WpGmtDateTime>,

    // Status & dates
    pub subscription_status: PurchaseSubscriptionStatus,
    pub expiry_status: PurchaseExpiryStatus,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub expiry_date: Option<WpGmtDateTime>,
    pub expiry_message: String,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub subscribed_date: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub renew_date: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub most_recent_renew_date: Option<WpGmtDateTime>,
    pub bill_period_days: u32,
    pub bill_period_label: String,
    /// Days until expiry; negative once the purchase is past its expiry date.
    pub days_until_expiry: i64,
    pub is_past_expiry_date: bool,
    /// Whether this purchase prevents the site from being deleted. This is the
    /// server-computed signal that replaces the v1.1 `active` delete-site gate.
    pub blocks_site_deletion: bool,

    // Auto-renew
    pub is_auto_renew_enabled: bool,
    pub can_disable_auto_renew: bool,
    pub can_reenable_auto_renewal: bool,
    pub can_explicit_renew: bool,

    // Capabilities
    pub is_cancelable: bool,
    pub is_refundable: bool,
    pub is_renewable: bool,
    pub is_renewal: bool,
    pub is_domain: bool,
    pub is_domain_registration: bool,
    pub is_plan: bool,
    pub is_locked: bool,
    pub is_hundred_year_domain: bool,
    pub is_trial_plan: bool,

    // Pricing
    pub amount: Decimal2,
    pub currency_code: CurrencyCode,
    pub currency_symbol: String,
    pub price_text: String,
    /// Price in the smallest currency unit (e.g. cents).
    pub price_integer: u64,
    pub regular_price_text: String,
    /// Regular (non-discounted) price in the smallest currency unit.
    pub regular_price_integer: u64,
    /// Domain name that consumed this bundle's included-domain credit, if any.
    pub included_domain: DomainName,
    pub included_domain_purchase_amount: Decimal2,

    // Refunds
    pub refund_amount: Decimal2,
    /// Refund amount in the smallest currency unit.
    pub refund_integer: u64,
    pub refund_text: String,
    pub refund_currency_symbol: String,
    pub refund_period_in_days: u32,
    /// Downgrade paths offered as refund alternatives, when applicable.
    #[serde(default)]
    #[uniffi(default = None)]
    pub refund_options: Option<Vec<RefundOption>>,
    pub total_refund_amount: Decimal2,
    /// Total refund amount in the smallest currency unit.
    pub total_refund_integer: u64,
    pub total_refund_text: String,
    pub total_refund_currency: CurrencyCode,

    // Payment
    /// Payment method; absent when the purchase is owned by another user.
    #[serde(default)]
    #[uniffi(default = None)]
    pub payment_type: Option<PaymentType>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_site_purchases_deserialization() {
        let file = File::open("tests/wpcom/purchases/site-purchases.json")
            .expect("Failed to open fixture");
        let purchases: Vec<SitePurchase> =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(purchases.len(), 1);
        let purchase = &purchases[0];

        assert_eq!(purchase.id, PurchaseId(12345678));
        assert_eq!(purchase.blog_id, WpComSiteId(11223344));
        assert_eq!(
            purchase.product_slug,
            ProductSlug("fake_bundle".to_string())
        );
        assert_eq!(
            purchase.subscription_status,
            PurchaseSubscriptionStatus::Active
        );
        assert_eq!(purchase.expiry_status, PurchaseExpiryStatus::ManualRenew);
        assert!(purchase.blocks_site_deletion);
        assert!(purchase.is_plan);
        assert_eq!(purchase.amount, Decimal2::from_hundredths(4800));
        assert_eq!(purchase.price_integer, 4800);
        assert_eq!(purchase.currency_code, CurrencyCode("USD".to_string()));
        assert_eq!(
            purchase.total_refund_currency,
            CurrencyCode("EUR".to_string())
        );
        assert_eq!(purchase.payment_type, Some(PaymentType::Credits));

        // `renew_date` is "" in the payload and must decode to None.
        assert_eq!(purchase.renew_date, None);
        // A populated ISO-8601-with-offset date decodes to Some.
        assert!(purchase.subscribed_date.is_some());
        // `refund_options` is [] in the payload (not null/absent).
        assert_eq!(purchase.refund_options, Some(vec![]));
    }
}
