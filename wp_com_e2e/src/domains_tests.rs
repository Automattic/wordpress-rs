use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::domains::{
    AllDomainsParams, CountryCode, DomainAvailabilityParams, DomainAvailabilityStatus,
    DomainListItemStatusType, DomainName, DomainSubtypeId,
};

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

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
                        | DomainListItemStatusType::Error => {}
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

    trials
}
