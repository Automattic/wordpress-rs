package rs.wordpress.api.kotlin

import uniffi.wp_api.PaidDomainSuggestion
import uniffi.wp_api.paidDomainSuggestionSaleCostDisplay

/**
 * Returns a formatted sale price string matching the server's
 * `combined_sale_cost_display` format (e.g. `"TL 47.50"`, `"TL 174"`),
 * or `null` if the domain is not on sale.
 */
val PaidDomainSuggestion.saleCostDisplay: String?
    get() = paidDomainSuggestionSaleCostDisplay(this)
