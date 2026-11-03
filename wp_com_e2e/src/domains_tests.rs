use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::api_error::{WpApiError, WpErrorCode};
use wp_api::wp_com::domains::{
    AllDomainsParams, CountryCode, DomainAvailabilityParams, DomainAvailabilityStatus,
    DomainListItemStatusType, DomainName, DomainSubtypeId, DomainSuggestionsParams,
    SetPrimaryDomainParams, SiteDomainType,
};

fn suggestions_params(query: &str) -> DomainSuggestionsParams {
    DomainSuggestionsParams {
        query: query.to_string(),
        quantity: Some(5),
        tlds: None,
        vendor: None,
        only_wordpressdotcom: None,
        include_wordpressdotcom: None,
        include_dotblogsubdomain: None,
        segment_id: None,
    }
}

/// Asserts that the suggestions endpoint rejects `params` with `expected_code`.
///
/// WordPress.com REST v1.x reports the code under `error`, where the WordPress
/// REST API uses `code`. Reading it back here pins that a rejection reaches
/// callers as `WpApiError::WpError` with the code intact, instead of an opaque
/// `UnknownError` they cannot branch on.
async fn expect_suggestions_error_code(
    ctx: &TestContext,
    params: DomainSuggestionsParams,
    expected_code: &str,
) -> Result<(), libtest_mimic::Failed> {
    match ctx.client.domains().suggestions(&params).await {
        Ok(response) => Err(format!(
            "expected the request to be rejected, got {} suggestions",
            response.data.len()
        )
        .into()),
        Err(WpApiError::WpError { error_code, .. }) => {
            let expected = WpErrorCode::CustomError(expected_code.to_string());
            if error_code == expected {
                Ok(())
            } else {
                Err(format!("expected error code {expected:?}, got {error_code:?}").into())
            }
        }
        Err(other) => {
            Err(format!("expected WpError carrying the API error code, got {other:?}").into())
        }
    }
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // GET /domains/suggestions/
    trials.push(Trial::test("domains::suggestions", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let suggestions = ctx
                    .client
                    .domains()
                    .suggestions(&suggestions_params("coolsite"))
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if suggestions.is_empty() {
                    return Err("expected non-empty suggestions list".into());
                }

                Ok(())
            })
        }
    }));

    // A query the API refuses. Callers branch on the code to show "no results
    // for that search" rather than a failure, so the code has to survive the
    // trip through `WpApiError`.
    trials.push(Trial::test("domains::suggestions_invalid_query", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                expect_suggestions_error_code(&ctx, suggestions_params(""), "invalid_query").await
            })
        }
    }));

    // A query the API accepts but has no domains for. Distinct from
    // `invalid_query` and from a genuine request failure.
    trials.push(Trial::test("domains::suggestions_empty_results", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                expect_suggestions_error_code(
                    &ctx,
                    suggestions_params(&"z".repeat(200)),
                    "empty_results",
                )
                .await
            })
        }
    }));

    // Quantity is validated at both ends of its range.
    trials.push(Trial::test(
        "domains::suggestions_quantity_below_minimum",
        {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    expect_suggestions_error_code(
                        &ctx,
                        DomainSuggestionsParams {
                            quantity: Some(0),
                            ..suggestions_params("coolsite")
                        },
                        "invalid_minimum_quantity",
                    )
                    .await
                })
            }
        },
    ));

    trials.push(Trial::test("domains::supported_countries", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let response = ctx
                    .client
                    .domains()
                    .supported_countries()
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if response.all.is_empty() {
                    return Err("expected non-empty `all` countries list".into());
                }

                // Verify no separator entries leaked through.
                let has_separator = response
                    .featured
                    .iter()
                    .chain(response.all.iter())
                    .any(|c| c.code.0.is_empty());
                if has_separator {
                    return Err("separator entry should be filtered out".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::supported_states_us", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let states = ctx
                    .client
                    .domains()
                    .supported_states(&CountryCode::from("US"))
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if states.is_empty() {
                    return Err("expected non-empty states list for US".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::supported_states_empty", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let states = ctx
                    .client
                    .domains()
                    .supported_states(&CountryCode::from("DE"))
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if !states.is_empty() {
                    return Err(format!(
                        "expected empty states list for DE, got {} entries",
                        states.len()
                    )
                    .into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::is_available_taken", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let availability = ctx
                    .client
                    .domains()
                    .is_available(
                        &DomainName("google.com".to_string()),
                        &DomainAvailabilityParams::default(),
                    )
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if availability.domain_name != "google.com" {
                    return Err(format!(
                        "expected domain_name 'google.com', got '{}'",
                        availability.domain_name
                    )
                    .into());
                }

                if availability.status == DomainAvailabilityStatus::Available {
                    return Err("expected google.com to not be available".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::is_available_likely_available", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let availability = ctx
                    .client
                    .domains()
                    .is_available(
                        &DomainName("xyzzy-test-unlikely-taken-2025.com".to_string()),
                        &DomainAvailabilityParams::default(),
                    )
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if availability.status != DomainAvailabilityStatus::Available {
                    return Err(format!(
                        "expected status Available, got {:?}",
                        availability.status
                    )
                    .into());
                }

                if !availability.supports_privacy {
                    return Err("expected .com domain to support privacy".into());
                }

                if availability.pricing.is_none() {
                    return Err("expected pricing for available domain".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::all_domains", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let response = ctx
                    .client
                    .domains()
                    .all_domains(&AllDomainsParams::default())
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if response.domains.is_empty() {
                    return Err("expected at least one domain".into());
                }

                // Every domain must have a non-empty domain name.
                for domain in &response.domains {
                    if domain.domain.0.is_empty() {
                        return Err("expected non-empty domain name".into());
                    }
                }

                // The test account should have at least one default_address
                let has_default = response
                    .domains
                    .iter()
                    .any(|d| d.subtype.id == DomainSubtypeId::DefaultAddress);
                if !has_default {
                    return Err("expected at least one domain with subtype default_address".into());
                }

                // All status types should be known values.
                for domain in &response.domains {
                    match &domain.domain_status.status_type {
                        DomainListItemStatusType::Success
                        | DomainListItemStatusType::Warning
                        | DomainListItemStatusType::Error
                        | DomainListItemStatusType::Alert
                        | DomainListItemStatusType::Neutral
                        | DomainListItemStatusType::Premium => {}
                        DomainListItemStatusType::Other(s) => {
                            return Err(format!(
                                "unexpected status type '{}' for domain '{}'",
                                s, domain.domain.0
                            )
                            .into());
                        }
                    }
                }

                Ok(())
            })
        }
    }));

    // GET /sites/{siteId}/domains/

    let site_id = ctx.site_id;

    trials.push(Trial::test("domains::site_domains", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let response = ctx
                    .client
                    .domains()
                    .site_domains(&site_id)
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if response.domains.is_empty() {
                    return Err("expected at least one domain for the test site".into());
                }

                // Every domain must have a non-empty domain name.
                for domain in &response.domains {
                    if domain.domain.0.is_empty() {
                        return Err("expected non-empty domain name".into());
                    }
                }

                // The test site should have at least one wpcom subdomain.
                let has_wpcom = response
                    .domains
                    .iter()
                    .any(|d| d.domain_type == SiteDomainType::Wpcom);
                if !has_wpcom {
                    return Err("expected at least one domain with type 'wpcom'".into());
                }

                // Exactly one domain should be primary.
                let primary_count = response
                    .domains
                    .iter()
                    .filter(|d| d.primary_domain == Some(true))
                    .count();
                if primary_count != 1 {
                    return Err(format!(
                        "expected exactly 1 primary domain, got {}",
                        primary_count
                    )
                    .into());
                }

                // All domain types should be known values.
                for domain in &response.domains {
                    match &domain.domain_type {
                        SiteDomainType::Registered
                        | SiteDomainType::Mapping
                        | SiteDomainType::Transfer
                        | SiteDomainType::Redirect
                        | SiteDomainType::Wpcom => {}
                        SiteDomainType::Other(s) => {
                            return Err(format!(
                                "unexpected domain type '{}' for domain '{}'",
                                s, domain.domain.0
                            )
                            .into());
                        }
                    }
                }

                Ok(())
            })
        }
    }));

    // POST /sites/{siteId}/domains/primary/
    //
    // This endpoint mutates the site's primary domain, so the test reads the
    // current primary and sets it again. This exercises the endpoint
    // idempotently without changing the test site's effective primary domain.
    trials.push(Trial::test("domains::set_primary", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let domains = ctx
                    .client
                    .domains()
                    .site_domains(&site_id)
                    .await
                    .map_err(|e| e.to_string())?
                    .data
                    .domains;

                let primary = match domains.iter().find(|d| d.primary_domain == Some(true)) {
                    Some(domain) => domain,
                    None => return Err("expected the test site to have a primary domain".into()),
                };

                let response = ctx
                    .client
                    .domains()
                    .set_primary(
                        &site_id,
                        &SetPrimaryDomainParams {
                            domain: primary.domain.clone(),
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if !response.success {
                    return Err("expected set_primary to report success".into());
                }

                Ok(())
            })
        }
    }));

    trials
}
