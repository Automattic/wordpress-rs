use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    decimal2::Decimal2,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::{
        CostOverrideCode, CouponCode, CurrencyCode, TimeSpanUnit,
        products::{BillPeriodDays, ProductId, ProductSlug, ProductTierId},
        purchases::PurchaseId,
    },
    wp_content_string_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

wp_content_string_id!(PlanFeatureKey);

/// Query parameters for `GET /sites/<site_id>/plans`.
#[derive(Debug, Default, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SitePlansParams {
    /// Coupon code to price the plans with. Plans the coupon doesn't apply to
    /// are returned at their normal price.
    #[uniffi(default = None)]
    pub coupon_code: Option<CouponCode>,
}

impl AppendUrlQueryPairs for SitePlansParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("coupon_code", self.coupon_code.as_ref());
    }
}

/// The plans available to a site, keyed by product ID.
///
/// The key always matches the entry's own `product_id`.
pub type SitePlansResponse = HashMap<ProductId, SitePlan>;

/// A plan available to a site, priced for that site and the requesting user.
///
/// Most fields describe the plan as an offer. The ones grouped into
/// [`SitePlanCurrentPlanInfo`], [`SitePlanSubscriptionInfo`] and
/// [`SitePlanTransition`] describe the plan's relationship to what the site is
/// on now, and are only present on the plans they apply to.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlan {
    pub product_id: ProductId,
    pub product_slug: ProductSlug,
    /// Localized plan name (e.g. `"WordPress.com Premium"`).
    pub product_name: String,
    /// Catalog tier this plan belongs to, which groups its monthly, yearly and
    /// multi-year variants.
    pub product_tier_id: ProductTierId,
    /// The plans sharing this plan's tier, limited to those the response also
    /// contains.
    pub product_tier_product_ids: Vec<ProductId>,

    pub currency_code: CurrencyCode,
    pub raw_price: Decimal2,
    /// `raw_price` in the currency's smallest unit (e.g. cents).
    pub raw_price_integer: u64,
    /// `raw_price` formatted for display, with trailing zeros stripped.
    pub formatted_price: String,
    /// The price before discounts, formatted for display. Formatted zero when
    /// the plan isn't discounted.
    pub formatted_original_price: String,
    pub raw_discount: Decimal2,
    /// `raw_discount` in the currency's smallest unit.
    pub raw_discount_integer: u64,
    pub formatted_discount: String,
    /// Localized explanation of the discount, when there is one.
    pub discount_reason: Option<String>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub cost_overrides: Vec<SitePlanCostOverride>,
    /// `None` for plans that don't set the flag either way.
    pub is_domain_upgrade: Option<bool>,
    pub interval: BillPeriodDays,

    /// This plan's position in the plans grid. `None` for plans the grid
    /// doesn't show.
    pub plan_card_order: Option<u32>,
    /// Short name for the plan card (e.g. `"Premium"`).
    pub plan_card_name: Option<String>,
    /// Localized one-line pitch for the plan.
    pub tagline: Option<String>,
    /// Localized labels for the plan card (e.g. `"Popular"`).
    #[serde(default)]
    #[uniffi(default = [])]
    pub badges: Vec<String>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub plan_card_features: Vec<SitePlanCardFeature>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub features_comparison: Vec<SitePlanComparisonGroup>,

    /// Set only on the plan the site is currently on.
    #[serde(flatten)]
    pub current_plan: Option<SitePlanCurrentPlanInfo>,
    /// Set only on the current plan, and only when it is paid.
    #[serde(flatten)]
    pub subscription: Option<SitePlanSubscriptionInfo>,
    /// Set when this plan has an introductory offer the user is eligible for.
    #[serde(flatten)]
    pub introductory_offer: Option<SitePlanIntroductoryOffer>,
    /// Set on every plan except the current one.
    #[serde(flatten)]
    pub transition: Option<SitePlanTransition>,
    /// Whether a trial of this plan can be started. Set on non-current, paid
    /// plans.
    pub can_start_trial: Option<bool>,
    /// Whether the request's `coupon_code` discounted this plan. Set only when
    /// a coupon was sent and this plan was eligible for it.
    pub has_sale_coupon: Option<bool>,
}

/// What the API reports about the plan a site is currently on.
///
/// A plan carrying this is active, though it may be past its expiry date and
/// inside its grace period — see [`SitePlanSubscriptionInfo::is_expired`].
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanCurrentPlanInfo {
    /// The purchase backing the plan. `None` on the free plan, which has no
    /// purchase behind it.
    #[serde(rename = "id")]
    pub purchase_id: Option<PurchaseId>,
    /// Whether the requesting user owns the plan's purchase. `None` on the free
    /// plan.
    pub user_is_owner: Option<bool>,
    /// Whether the plan still carries an unclaimed free-domain credit.
    pub has_domain_credit: bool,
}

/// Purchase details for the site's current paid plan.
///
/// Absent on the free plan, which has no purchase, and on every plan the site
/// isn't currently on.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanSubscriptionInfo {
    /// When the plan expires. `None` for partner-provisioned plans, which the
    /// backend reports as never expiring.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub expiry: Option<WpGmtDateTime>,
    /// Deprecated by the backend, which now always reports it as `expiry`.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub user_facing_expiry: Option<WpGmtDateTime>,
    /// Whether the plan is past its expiry date. A current plan that is expired
    /// is still active, but inside its grace period; once that ends the plan
    /// stops being reported as current.
    pub is_expired: bool,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub subscribed_date: Option<WpGmtDateTime>,
    pub free_trial: bool,
    pub auto_renew: bool,
    /// When auto-renewal will next be attempted.
    pub auto_renew_date: WpGmtDateTime,
    /// Whether the plan's free-domain credit has already been claimed.
    pub has_redeemed_domain_credit: bool,
    /// The Jetpack partner that provisioned the plan. Empty when no partner
    /// did.
    pub partner_name: String,
    /// Whether the customer has asked to move to a cheaper plan at the end of
    /// the current term.
    pub is_delayed_downgrade_pending: bool,
    /// The plan the site will move to at renewal. Set only while
    /// `is_delayed_downgrade_pending`.
    pub delayed_downgrade_to_product_slug: Option<ProductSlug>,
}

/// Introductory pricing for a plan, applied for a limited number of intervals
/// before it renews at its normal price.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanIntroductoryOffer {
    /// The offer price, formatted for display.
    #[serde(rename = "introductory_offer_formatted_price")]
    pub formatted_price: String,
    #[serde(rename = "introductory_offer_raw_price")]
    pub raw_price: Decimal2,
    /// `raw_price` in the currency's smallest unit.
    #[serde(rename = "introductory_offer_raw_price_integer")]
    pub raw_price_integer: u64,
    #[serde(rename = "introductory_offer_interval_unit")]
    pub interval_unit: TimeSpanUnit,
    /// How many `interval_unit`s the offer price covers.
    #[serde(rename = "introductory_offer_interval_count")]
    pub interval_count: u32,
    /// When the offer ends. Set only on the current plan, and only when the
    /// site has a purchase for it.
    #[serde(
        rename = "introductory_offer_end_date",
        default,
        deserialize_with = "deserialize_optional_wp_gmt_date_time"
    )]
    #[uniffi(default = None)]
    pub end_date: Option<WpGmtDateTime>,
}

/// Whether a site can move from its current plan to this one, along the same
/// upgrade and downgrade paths the shopping cart uses.
///
/// Absent on the current plan itself.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanTransition {
    pub available_for_upgrade: bool,
    pub available_for_downgrade: bool,
}

/// An adjustment applied to a plan's price.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanCostOverride {
    pub override_code: CostOverrideCode,
    /// Localized explanation of the adjustment.
    pub human_readable_reason: String,
    pub old_price: Decimal2,
    pub new_price: Decimal2,
    /// Whether this replaces the plan's base price rather than discounting it.
    /// Adjustments that do aren't meant to be shown to users — they define the
    /// price other adjustments apply to.
    pub does_override_original_cost: bool,
    /// Discount percentage, where `10` means 10% off. Zero for adjustments
    /// expressed as an amount rather than a percentage.
    pub percentage: Decimal2,
    /// Whether the discount covers only the first unit of a multi-unit
    /// purchase, such as the first year of a multi-year term.
    pub first_unit_only: bool,
}

/// A feature listed on a plan card.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanCardFeature {
    /// Localized description of the feature.
    pub text: String,
    /// Whether this plan includes it.
    pub available: bool,
}

/// A group of rows in the plans comparison grid.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanComparisonGroup {
    /// Localized group heading (e.g. `"Essential features"`).
    pub group: String,
    pub features: Vec<SitePlanComparisonFeature>,
}

/// A single row in the plans comparison grid.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct SitePlanComparisonFeature {
    pub key: PlanFeatureKey,
    /// Localized feature name.
    pub title: String,
    /// The tiers this feature is shown for. Empty when it isn't restricted to
    /// any.
    #[serde(default)]
    #[uniffi(default = [])]
    pub tiers: Vec<ProductTierId>,
    /// The billing periods this feature is restricted to. Empty when it applies
    /// to all of them.
    #[serde(default)]
    #[uniffi(default = [])]
    pub billing_periods: Vec<BillPeriodDays>,
    /// Per-tier display values, keyed by tier — used where a feature differs by
    /// plan rather than being present or absent (e.g. storage size).
    #[serde(default)]
    pub tier_values: HashMap<ProductTierId, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::fs::File;

    fn read_fixture(path: &str) -> SitePlansResponse {
        let file = File::open(path).expect("Failed to open file");
        serde_json::from_reader(file).expect("Unable to parse JSON")
    }

    fn plan(plans: &SitePlansResponse, product_id: u64) -> &SitePlan {
        plans
            .get(&ProductId(product_id))
            .unwrap_or_else(|| panic!("fixture should contain product {product_id}"))
    }

    #[rstest]
    #[case("tests/wpcom/site_plans/free-plan-site.json")]
    #[case("tests/wpcom/site_plans/paid-plan-site.json")]
    #[case("tests/wpcom/site_plans/jetpack-site.json")]
    fn test_site_plans_deserialization(#[case] json_file_path: &str) {
        let plans = read_fixture(json_file_path);
        assert!(!plans.is_empty());
    }

    /// The response is keyed by product ID as a decimal string, and the key
    /// always repeats the entry's own `product_id`. Callers index by the key, so
    /// a mismatch would silently hand back the wrong plan.
    #[rstest]
    #[case("tests/wpcom/site_plans/free-plan-site.json")]
    #[case("tests/wpcom/site_plans/paid-plan-site.json")]
    #[case("tests/wpcom/site_plans/jetpack-site.json")]
    fn test_map_key_matches_product_id(#[case] json_file_path: &str) {
        for (product_id, plan) in read_fixture(json_file_path) {
            assert_eq!(product_id, plan.product_id);
        }
    }

    /// Exactly one plan carries the current-plan group, and the conditional
    /// groups that describe a plan's relationship to it are mutually exclusive
    /// with it.
    #[rstest]
    #[case("tests/wpcom/site_plans/free-plan-site.json", 9001)]
    #[case("tests/wpcom/site_plans/paid-plan-site.json", 9005)]
    #[case("tests/wpcom/site_plans/jetpack-site.json", 9101)]
    fn test_exactly_one_current_plan(#[case] json_file_path: &str, #[case] expected_current: u64) {
        let plans = read_fixture(json_file_path);

        let current: Vec<_> = plans
            .iter()
            .filter(|(_, plan)| plan.current_plan.is_some())
            .map(|(product_id, _)| *product_id)
            .collect();
        assert_eq!(current, vec![ProductId(expected_current)]);

        // `transition` is the complement of `current_plan`: the API reports
        // upgrade/downgrade availability for every plan except the current one.
        for (product_id, plan) in &plans {
            assert_eq!(
                plan.current_plan.is_some(),
                plan.transition.is_none(),
                "product {product_id} should carry exactly one of current_plan/transition"
            );
        }
    }

    /// The free plan is current but has no purchase behind it, so the
    /// subscription group is absent and the two ownership fields are null.
    #[test]
    fn test_current_free_plan_has_no_subscription() {
        let plans = read_fixture("tests/wpcom/site_plans/free-plan-site.json");
        let free = plan(&plans, 9001);

        assert_eq!(free.product_slug, ProductSlug("fake_free_plan".to_string()));
        assert_eq!(free.interval, BillPeriodDays(-1));
        assert_eq!(free.raw_price, Decimal2::from_hundredths(0));

        let current = free
            .current_plan
            .as_ref()
            .expect("free plan should be the current plan");
        assert!(current.purchase_id.is_none());
        assert!(current.user_is_owner.is_none());
        assert!(!current.has_domain_credit);

        assert!(
            free.subscription.is_none(),
            "the free plan has no purchase, so it carries no subscription details"
        );
        assert!(
            free.can_start_trial.is_none(),
            "can_start_trial is only reported for non-current paid plans"
        );
    }

    /// The current *paid* plan is the only entry that carries the subscription
    /// group, which is the whole reason it's modelled separately from
    /// `current_plan`.
    #[test]
    fn test_current_paid_plan_subscription_details() {
        let plans = read_fixture("tests/wpcom/site_plans/paid-plan-site.json");
        let premium = plan(&plans, 9005);

        let current = premium
            .current_plan
            .as_ref()
            .expect("the 2-year plan should be the current plan");
        assert_eq!(current.purchase_id, Some(PurchaseId(55501234)));
        assert_eq!(current.user_is_owner, Some(false));

        let subscription = premium
            .subscription
            .as_ref()
            .expect("a paid current plan should carry subscription details");
        assert_eq!(
            subscription.expiry,
            Some("2029-03-15T00:00:00+00:00".parse().expect("valid date"))
        );
        assert_eq!(subscription.user_facing_expiry, subscription.expiry);
        assert!(!subscription.is_expired);
        assert!(!subscription.auto_renew);
        assert!(!subscription.free_trial);
        assert_eq!(
            subscription.partner_name, "",
            "not provisioned by a partner"
        );
        assert!(!subscription.is_delayed_downgrade_pending);
        assert!(subscription.delayed_downgrade_to_product_slug.is_none());

        assert_eq!(premium.interval, BillPeriodDays(730));

        // Every other plan in the response is missing both groups.
        let free = plan(&plans, 9001);
        assert!(free.current_plan.is_none());
        assert!(free.subscription.is_none());
    }

    #[test]
    fn test_cost_overrides() {
        let plans = read_fixture("tests/wpcom/site_plans/paid-plan-site.json");
        let three_year = plan(&plans, 9006);

        assert_eq!(three_year.cost_overrides.len(), 1);
        let override_ = &three_year.cost_overrides[0];
        assert_eq!(
            override_.override_code,
            CostOverrideCode("fake-plan-proration".to_string())
        );
        assert_eq!(
            override_.human_readable_reason,
            "Fake prorated balance from previous plan"
        );
        assert!(!override_.does_override_original_cost);
        assert_eq!(override_.percentage, Decimal2::from_hundredths(0));
        assert!(!override_.first_unit_only);
        assert!(override_.new_price < override_.old_price);

        // Plans without an adjustment get an empty list, not a missing field.
        assert!(plan(&plans, 9001).cost_overrides.is_empty());
    }

    #[test]
    fn test_introductory_offer() {
        let plans = read_fixture("tests/wpcom/site_plans/jetpack-site.json");

        let yearly = plan(&plans, 9102)
            .introductory_offer
            .as_ref()
            .expect("the yearly add-on should have an introductory offer");
        assert_eq!(yearly.interval_unit, TimeSpanUnit::Year);
        assert_eq!(yearly.interval_count, 1);
        assert!(
            yearly.end_date.is_none(),
            "the end date is only reported for the current plan's own offer"
        );

        let bi_yearly = plan(&plans, 9103)
            .introductory_offer
            .as_ref()
            .expect("the two-year add-on should have an introductory offer");
        assert_eq!(bi_yearly.interval_count, 2);

        assert!(
            plan(&plans, 9101).introductory_offer.is_none(),
            "the free plan has no introductory offer"
        );
    }

    /// The comparison grid keys its per-tier values by product tier ID, which
    /// arrives as a JSON object key (a string) while `tiers` and
    /// `product_tier_id` arrive as numbers. Both have to land on the same type.
    #[test]
    fn test_comparison_features_tier_values() {
        let plans = read_fixture("tests/wpcom/site_plans/free-plan-site.json");
        let free = plan(&plans, 9001);

        let groups = &free.features_comparison;
        assert!(!groups.is_empty());

        let features: Vec<_> = groups.iter().flat_map(|g| &g.features).collect();

        // `tier_values` keys arrive as strings, so finding the free plan's own
        // numeric tier in them proves the key type parses both encodings.
        let with_tier_values = features
            .iter()
            .find(|f| f.tier_values.contains_key(&free.product_tier_id))
            .expect("fixture should include a feature with a value for the free tier");
        assert!(
            !with_tier_values.tier_values[&free.product_tier_id].is_empty(),
            "a per-tier value should be a non-empty display string"
        );

        let with_billing_periods = features
            .iter()
            .find(|f| !f.billing_periods.is_empty())
            .expect("fixture should include a feature restricted to a billing period");
        assert!(
            with_billing_periods
                .billing_periods
                .contains(&BillPeriodDays(365))
        );

        // `tiers` holds the same identifiers the grid is keyed by, but arrives as
        // numbers rather than object keys.
        assert!(
            features
                .iter()
                .any(|f| f.tiers.contains(&free.product_tier_id)),
            "fixture should include a feature shown for the free tier"
        );
    }

    /// The marketing fields are absent rather than empty for plans the grid
    /// doesn't render, so they need `serde(default)` to survive.
    #[test]
    fn test_plan_without_marketing_fields() {
        let plans = read_fixture("tests/wpcom/site_plans/free-plan-site.json");

        let bare = plan(&plans, 9004);
        assert!(bare.badges.is_empty());
        assert!(bare.plan_card_features.is_empty());
        assert!(bare.features_comparison.is_empty());
        assert!(bare.plan_card_name.is_none());
        assert!(bare.plan_card_order.is_none());
        // Taglines are assigned per product, independently of whether the plan
        // appears in the grid, so this plan has one without any card fields.
        assert!(bare.tagline.is_some());

        // A plan can have a comparison grid but no card features.
        let partial = plan(&plans, 9003);
        assert!(partial.plan_card_features.is_empty());
        assert!(!partial.features_comparison.is_empty());

        let full = plan(&plans, 9002);
        assert!(!full.badges.is_empty());
        assert!(!full.plan_card_features.is_empty());
    }

    #[test]
    fn test_product_tier_grouping() {
        let plans = read_fixture("tests/wpcom/site_plans/paid-plan-site.json");
        let premium_2y = plan(&plans, 9005);

        // The tier groups a plan's billing-term variants, so the plan is always
        // listed among its own tier's products.
        assert!(
            premium_2y
                .product_tier_product_ids
                .contains(&premium_2y.product_id)
        );
        assert_eq!(
            plan(&plans, 9006).product_tier_id,
            premium_2y.product_tier_id,
            "the 2-year and 3-year Premium plans share a tier"
        );
    }
}
