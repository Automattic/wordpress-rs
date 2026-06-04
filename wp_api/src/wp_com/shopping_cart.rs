use crate::{
    date::WpGmtDateTime,
    decimal2::Decimal2,
    wp_com::{
        CurrencyCode, WpComSiteId, domains::DomainName, products::ProductId,
        subscribers::SubscriptionId,
    },
};
use serde::{Deserialize, Serialize, de};
use std::fmt;

uniffi::custom_newtype!(CartItemId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CartItemId(pub String);

uniffi::custom_newtype!(BillingPlanId, String);
/// Billing plan identifier, returned as a numeric string by the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BillingPlanId(pub String);

uniffi::custom_newtype!(PurchaseId, u64);
/// Identifies an existing purchase/subscription for renewals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PurchaseId(pub u64);

/// Identifies whose cart this is: a specific site or no site.
///
/// Used both as a URL path segment (`/me/shopping-cart/<cart_key>`)
/// and as the response `cart_key` field (number or `"no-site"`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum CartKey {
    Site { id: WpComSiteId },
    NoSite,
}

impl fmt::Display for CartKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CartKey::Site { id } => write!(f, "{}", id),
            CartKey::NoSite => write!(f, "no-site"),
        }
    }
}

impl Serialize for CartKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CartKey::Site { id } => serializer.serialize_u64(id.0),
            CartKey::NoSite => serializer.serialize_str("no-site"),
        }
    }
}

impl<'de> Deserialize<'de> for CartKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(CartKeyVisitor)
    }
}

/// Custom visitor because the API's `cart_key` field is polymorphic: a
/// numeric site ID (`12345678`) or the string `"no-site"`.
///
/// An `#[serde(untagged)]` enum could handle this automatically, but
/// `NoSite` would need to wrap a `String` instead of being a unit variant,
/// and it would accept any string value. The visitor keeps `NoSite` as a
/// clean unit variant and rejects unexpected string values.
struct CartKeyVisitor;

impl<'de> de::Visitor<'de> for CartKeyVisitor {
    type Value = CartKey;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a site ID (number) or \"no-site\"")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(CartKey::Site { id: WpComSiteId(v) })
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v == "no-site" {
            Ok(CartKey::NoSite)
        } else {
            Err(E::invalid_value(
                de::Unexpected::Str(v),
                &"\"no-site\" or a numeric site ID",
            ))
        }
    }
}

/// Parameters for `POST /me/shopping-cart/<cart_key>`.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct CreateShoppingCartParams {
    /// Whether this is a temporary cart (not persisted).
    pub temporary: bool,
    /// Products to add to the cart.
    pub products: Vec<CreateShoppingCartProduct>,
}

/// A product to add to the shopping cart.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct CreateShoppingCartProduct {
    /// Product ID (from the `/products` endpoint).
    pub product_id: ProductId,
    /// Domain name being purchased (required for domain products).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// Extra options (required for domain products).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub extra: Option<CreateShoppingCartProductExtra>,
}

/// Extra options for a domain product in the cart creation request.
#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct CreateShoppingCartProductExtra {
    /// Whether WHOIS privacy protection is enabled.
    pub privacy: bool,
}

/// Response from `POST /me/shopping-cart/<cart_key>`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCart {
    pub cart_generated_at_timestamp: u64,
    pub blog_id: WpComSiteId,
    pub cart_key: CartKey,
    pub coupon: String,
    pub is_coupon_applied: bool,
    pub has_auto_renew_coupon_been_automatically_applied: bool,
    pub next_domain_is_free: bool,
    pub next_domain_condition: String,
    pub products: Vec<ShoppingCartProduct>,
    pub unmerged_products: Vec<ShoppingCartProduct>,
    pub total_cost: Decimal2,
    pub currency: CurrencyCode,
    pub total_cost_integer: u64,
    pub temporary: bool,
    pub tax: ShoppingCartTax,
    pub coupon_savings_total_integer: u64,
    pub sub_total_with_taxes_integer: u64,
    pub sub_total_integer: u64,
    pub total_tax: Decimal2,
    pub total_tax_integer: u64,
    pub credits: Decimal2,
    pub credits_integer: u64,
    pub allowed_payment_methods: Vec<String>,
    pub is_gift_purchase: bool,
    pub messages: ShoppingCartMessages,
    /// The domain bundled with a plan, if any.
    #[serde(default)]
    #[uniffi(default = None)]
    pub bundled_domain: Option<DomainName>,
}

/// A product in the shopping cart response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartProduct {
    pub cart_item_id: CartItemId,
    pub product_id: ProductId,
    pub billing_plan_id: BillingPlanId,
    pub product_name: String,
    pub product_name_en: String,
    pub product_slug: String,
    /// Domain name for domain products, empty string for other products.
    pub meta: String,
    pub cost: Decimal2,
    pub currency: CurrencyCode,
    pub volume: u32,
    #[serde(default)]
    #[uniffi(default = None)]
    pub quantity: Option<u32>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub current_quantity: Option<u32>,
    pub coupon_savings_integer: u64,
    pub is_sale_coupon_applied: bool,
    pub extra: ShoppingCartProductExtra,
    pub bill_period: String,
    pub months_per_bill_period: u32,
    pub is_domain_registration: bool,
    pub time_added_to_cart: u64,
    pub is_bundled: bool,
    pub item_original_cost: Decimal2,
    pub item_original_cost_integer: u64,
    pub item_original_monthly_cost_integer: u64,
    pub item_original_cost_for_quantity_one_integer: u64,
    pub item_subtotal_monthly_cost_integer: u64,
    pub item_original_subtotal: Decimal2,
    pub item_original_subtotal_integer: u64,
    pub item_subtotal: Decimal2,
    pub item_subtotal_integer: u64,
    pub item_tax: Decimal2,
    pub item_tax_rate: f64,
    pub item_total: Decimal2,
    pub item_total_integer: u64,
    pub subscription_id: SubscriptionId,
    pub is_renewal: bool,
    pub is_renewal_and_will_auto_renew: bool,
    pub is_one_time_purchase: bool,
    #[serde(default)]
    #[uniffi(default = [])]
    pub cost_overrides: Vec<ShoppingCartCostOverride>,
    pub is_gift_purchase: bool,
    #[serde(default)]
    #[uniffi(default = [])]
    pub product_variants: Vec<ShoppingCartProductVariant>,
    pub is_included_for_100yearplan: bool,
    #[serde(default)]
    #[uniffi(default = None)]
    pub stored_details_id: Option<String>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub subscription_current_expiry_date: Option<WpGmtDateTime>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub subscription_post_purchase_expiry_date: Option<WpGmtDateTime>,
}

/// Extra fields on a shopping cart product in the response.
///
/// The set of fields varies by product type — domain products include
/// privacy and registrar fields, plan products include `domain_to_bundle`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartProductExtra {
    #[serde(default)]
    #[uniffi(default = None)]
    pub privacy: Option<bool>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub added_from_shopping_cart: Option<bool>,
    #[serde(default, rename = "purchaseId")]
    #[uniffi(default = None)]
    pub purchase_id: Option<PurchaseId>,
    #[serde(default, rename = "purchaseType")]
    #[uniffi(default = None)]
    pub purchase_type: Option<String>,
    /// Domain registration details, present only for domain registration
    /// products (e.g. `domain_reg`). Absent for plans and `domain_map`.
    #[serde(flatten)]
    pub domain_registration_info: Option<DomainRegistrationExtraInfo>,
    #[serde(default)]
    #[uniffi(default = None)]
    pub domain_to_bundle: Option<DomainName>,
}

/// Domain registration-specific extra fields on a cart product.
///
/// Present when the product is a domain registration (`domain_reg`);
/// absent for plans and domain mappings. The required `registrar` field
/// acts as the anchor — when it's missing, serde deserializes the whole
/// group as `None`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainRegistrationExtraInfo {
    pub registrar: String,
    pub domain_registration_agreement_url: String,
    pub privacy_available: bool,
    pub premium: bool,
}

/// A pricing variant for a product in the cart.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartProductVariant {
    pub price_before_discounts_integer: u64,
    pub introductory_offer_discount_integer: u64,
    pub price_integer: u64,
    pub bill_period_in_months: u32,
    pub currency: CurrencyCode,
    pub product_id: ProductId,
    pub product_slug: String,
    pub volume: u32,
}

/// A cost override applied to a bundled product (e.g. free domain with plan).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartCostOverride {
    pub old_price: Decimal2,
    pub old_price_integer: u64,
    pub new_price: Decimal2,
    pub new_price_integer: u64,
    pub old_subtotal: Decimal2,
    pub old_subtotal_integer: u64,
    pub new_subtotal: Decimal2,
    pub new_subtotal_integer: u64,
    pub override_code: String,
    pub does_override_original_cost: bool,
    pub percentage: u32,
    pub first_unit_only: bool,
    pub human_readable_reason: String,
}

/// Tax information for the shopping cart.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartTax {
    pub display_taxes: bool,
}

/// Messages returned with the shopping cart response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartMessages {
    #[serde(default)]
    #[uniffi(default = [])]
    pub errors: Vec<ShoppingCartMessage>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub success: Vec<ShoppingCartMessage>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub persistent_errors: Vec<ShoppingCartMessage>,
}

/// An individual error or success message in the cart response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ShoppingCartMessage {
    pub code: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::fs::File;

    #[rstest]
    #[case("tests/wpcom/shopping_cart/cart-with-site.json")]
    #[case("tests/wpcom/shopping_cart/cart-no-site.json")]
    #[case("tests/wpcom/shopping_cart/cart-with-plan.json")]
    #[case("tests/wpcom/shopping_cart/cart-invalid-product.json")]
    fn test_shopping_cart_deserialization(#[case] json_file_path: &str) {
        let file = File::open(json_file_path).expect("Failed to open file");
        let _cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_cart_with_site_details() {
        let file = File::open("tests/wpcom/shopping_cart/cart-with-site.json")
            .expect("Failed to open file");
        let cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(cart.blog_id, WpComSiteId(12345678));
        assert_eq!(
            cart.cart_key,
            CartKey::Site {
                id: WpComSiteId(12345678)
            }
        );
        assert_eq!(cart.products.len(), 1);
        assert_eq!(cart.unmerged_products.len(), 2);
        assert!(cart.temporary);
        assert_eq!(cart.currency, CurrencyCode("USD".to_string()));
        assert!(!cart.is_gift_purchase);

        let product = &cart.products[0];
        assert_eq!(product.product_id, ProductId(6));
        assert_eq!(product.product_slug, "domain_reg");
        assert_eq!(product.meta, "fake-test-domain.com");
        assert!(product.is_domain_registration);
        assert!(!product.is_bundled);
        assert_eq!(product.extra.privacy, Some(true));
        let domain_info = product
            .extra
            .domain_registration_info
            .as_ref()
            .expect("domain_reg product should have domain_registration_info");
        assert_eq!(domain_info.registrar, "FAKE_REGISTRAR");
        assert!(domain_info.privacy_available);
        assert!(!domain_info.premium);
    }

    #[test]
    fn test_cart_no_site_details() {
        let file =
            File::open("tests/wpcom/shopping_cart/cart-no-site.json").expect("Failed to open file");
        let cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(cart.blog_id, WpComSiteId(0));
        assert_eq!(cart.cart_key, CartKey::NoSite);
        assert_eq!(cart.products.len(), 1);
        assert_eq!(cart.unmerged_products.len(), 2);
        assert_eq!(cart.products[0].extra.privacy, Some(false));
    }

    #[test]
    fn test_cart_with_plan_details() {
        let file = File::open("tests/wpcom/shopping_cart/cart-with-plan.json")
            .expect("Failed to open file");
        let cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(cart.products.len(), 2);
        assert_eq!(
            cart.bundled_domain.as_ref().map(|d| d.0.as_str()),
            Some("fake-plan-domain.com")
        );

        // Plan product — no domain registration info.
        let plan = &cart.products[0];
        assert_eq!(plan.product_id, ProductId(1009));
        assert_eq!(plan.product_slug, "personal-bundle");
        assert!(!plan.is_domain_registration);
        assert!(plan.extra.domain_registration_info.is_none());
        assert_eq!(
            plan.extra.domain_to_bundle.as_ref().map(|d| d.0.as_str()),
            Some("fake-plan-domain.com")
        );

        // Bundled domain product (free with plan).
        let domain = &cart.products[1];
        assert_eq!(domain.product_id, ProductId(6));
        assert!(domain.is_bundled);
        assert_eq!(domain.item_subtotal, Decimal2::from_hundredths(0));
        assert!(!domain.cost_overrides.is_empty());
        assert_eq!(
            domain.cost_overrides[0].override_code,
            "bundled-domain-credit"
        );
        assert_eq!(
            domain.cost_overrides[0].human_readable_reason,
            "Free domain for first year"
        );
    }

    #[test]
    fn test_cart_invalid_product_error() {
        let file = File::open("tests/wpcom/shopping_cart/cart-invalid-product.json")
            .expect("Failed to open file");
        let cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(cart.products.is_empty());
        assert!(!cart.messages.errors.is_empty());
        assert_eq!(cart.messages.errors[0].code, "invalid-product-id");
    }

    #[test]
    fn test_cart_key_serialization_roundtrip() {
        let site_key = CartKey::Site {
            id: WpComSiteId(12345),
        };
        let json = serde_json::to_string(&site_key).expect("should serialize");
        assert_eq!(json, "12345");
        let deserialized: CartKey = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized, site_key);

        let no_site_key = CartKey::NoSite;
        let json = serde_json::to_string(&no_site_key).expect("should serialize");
        assert_eq!(json, "\"no-site\"");
        let deserialized: CartKey = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(deserialized, no_site_key);
    }

    #[test]
    fn test_cart_key_display() {
        assert_eq!(
            CartKey::Site {
                id: WpComSiteId(12345)
            }
            .to_string(),
            "12345"
        );
        assert_eq!(CartKey::NoSite.to_string(), "no-site");
    }

    #[test]
    fn test_product_variants() {
        let file = File::open("tests/wpcom/shopping_cart/cart-with-site.json")
            .expect("Failed to open file");
        let cart: ShoppingCart = serde_json::from_reader(file).expect("Unable to parse JSON");

        let product = &cart.products[0];
        assert!(!product.product_variants.is_empty());
        let first_variant = &product.product_variants[0];
        assert_eq!(first_variant.bill_period_in_months, 12);
        assert_eq!(first_variant.product_id, ProductId(6));
        assert_eq!(first_variant.currency, CurrencyCode("USD".to_string()));
    }
}
