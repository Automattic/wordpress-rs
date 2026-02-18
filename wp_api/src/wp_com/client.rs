use super::endpoint::{
    followers_endpoint::{FollowersRequestBuilder, FollowersRequestExecutor},
    jetpack_connection_endpoint::{
        JetpackConnectionRequestBuilder, JetpackConnectionRequestExecutor,
    },
    me_endpoint::{MeRequestBuilder, MeRequestExecutor},
    oauth2::{Oauth2RequestBuilder, Oauth2RequestExecutor},
    stats_city_views_endpoint::{StatsCityViewsRequestBuilder, StatsCityViewsRequestExecutor},
    stats_clicks_endpoint::{StatsClicksRequestBuilder, StatsClicksRequestExecutor},
    stats_country_views_endpoint::{
        StatsCountryViewsRequestBuilder, StatsCountryViewsRequestExecutor,
    },
    stats_devices_browser_endpoint::{
        StatsDevicesBrowserRequestBuilder, StatsDevicesBrowserRequestExecutor,
    },
    stats_devices_platform_endpoint::{
        StatsDevicesPlatformRequestBuilder, StatsDevicesPlatformRequestExecutor,
    },
    stats_devices_screensize_endpoint::{
        StatsDevicesScreensizeRequestBuilder, StatsDevicesScreensizeRequestExecutor,
    },
    stats_referrers_endpoint::{StatsReferrersRequestBuilder, StatsReferrersRequestExecutor},
    stats_region_views_endpoint::{
        StatsRegionViewsRequestBuilder, StatsRegionViewsRequestExecutor,
    },
    stats_top_authors_endpoint::{StatsTopAuthorsRequestBuilder, StatsTopAuthorsRequestExecutor},
    stats_top_posts_endpoint::{StatsTopPostsRequestBuilder, StatsTopPostsRequestExecutor},
    stats_visits_endpoint::{StatsVisitsRequestBuilder, StatsVisitsRequestExecutor},
    subscribers_endpoint::{SubscribersRequestBuilder, SubscribersRequestExecutor},
    support_bots_endpoint::{SupportBotsRequestBuilder, SupportBotsRequestExecutor},
    support_eligibility_endpoint::{
        SupportEligibilityRequestBuilder, SupportEligibilityRequestExecutor,
    },
    support_tickets_endpoint::{SupportTicketsRequestBuilder, SupportTicketsRequestExecutor},
};
use crate::{
    api_client::WpApiClientDelegate,
    api_client_generate_api_client, api_client_generate_endpoint_impl,
    api_client_generate_request_builder,
    auth::WpAuthenticationProvider,
    request::endpoint::ApiUrlResolver,
    wp_com::endpoint::{
        WpComApiClientInternalUrlResolver,
        languages_endpoint::{LanguagesRequestBuilder, LanguagesRequestExecutor},
        sites_endpoint::{SitesRequestBuilder, SitesRequestExecutor},
    },
};
use std::sync::Arc;

pub struct WpComApiRequestBuilder {
    followers: Arc<FollowersRequestBuilder>,
    jetpack_connection: Arc<JetpackConnectionRequestBuilder>,
    languages: Arc<LanguagesRequestBuilder>,
    me: Arc<MeRequestBuilder>,
    oauth2: Arc<Oauth2RequestBuilder>,
    sites: Arc<SitesRequestBuilder>,
    stats_city_views: Arc<StatsCityViewsRequestBuilder>,
    stats_clicks: Arc<StatsClicksRequestBuilder>,
    stats_country_views: Arc<StatsCountryViewsRequestBuilder>,
    stats_devices_browser: Arc<StatsDevicesBrowserRequestBuilder>,
    stats_devices_platform: Arc<StatsDevicesPlatformRequestBuilder>,
    stats_devices_screensize: Arc<StatsDevicesScreensizeRequestBuilder>,
    stats_referrers: Arc<StatsReferrersRequestBuilder>,
    stats_region_views: Arc<StatsRegionViewsRequestBuilder>,
    stats_top_authors: Arc<StatsTopAuthorsRequestBuilder>,
    stats_top_posts: Arc<StatsTopPostsRequestBuilder>,
    stats_visits: Arc<StatsVisitsRequestBuilder>,
    subscribers: Arc<SubscribersRequestBuilder>,
    support_bots: Arc<SupportBotsRequestBuilder>,
    support_eligibility: Arc<SupportEligibilityRequestBuilder>,
    support_tickets: Arc<SupportTicketsRequestBuilder>,
}

impl WpComApiRequestBuilder {
    pub fn new(auth_provider: Arc<WpAuthenticationProvider>) -> Self {
        let api_url_resolver: Arc<dyn ApiUrlResolver> =
            Arc::new(WpComApiClientInternalUrlResolver::default());
        api_client_generate_request_builder!(
            api_url_resolver,
            auth_provider;
            followers,
            jetpack_connection,
            languages,
            me,
            oauth2,
            sites,
            stats_city_views,
            stats_clicks,
            stats_country_views,
            stats_devices_browser,
            stats_devices_platform,
            stats_devices_screensize,
            stats_referrers,
            stats_region_views,
            stats_top_authors,
            stats_top_posts,
            stats_visits,
            subscribers,
            support_bots,
            support_eligibility,
            support_tickets
        )
    }
}

#[derive(uniffi::Object)]
struct UniffiWpComApiClient {
    inner: WpComApiClient,
}

#[uniffi::export]
impl UniffiWpComApiClient {
    #[uniffi::constructor]
    fn new(delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: WpComApiClient::new(delegate),
        }
    }
}

pub struct WpComApiClient {
    followers: Arc<FollowersRequestExecutor>,
    jetpack_connection: Arc<JetpackConnectionRequestExecutor>,
    languages: Arc<LanguagesRequestExecutor>,
    me: Arc<MeRequestExecutor>,
    oauth2: Arc<Oauth2RequestExecutor>,
    sites: Arc<SitesRequestExecutor>,
    stats_city_views: Arc<StatsCityViewsRequestExecutor>,
    stats_clicks: Arc<StatsClicksRequestExecutor>,
    stats_country_views: Arc<StatsCountryViewsRequestExecutor>,
    stats_devices_browser: Arc<StatsDevicesBrowserRequestExecutor>,
    stats_devices_platform: Arc<StatsDevicesPlatformRequestExecutor>,
    stats_devices_screensize: Arc<StatsDevicesScreensizeRequestExecutor>,
    stats_referrers: Arc<StatsReferrersRequestExecutor>,
    stats_region_views: Arc<StatsRegionViewsRequestExecutor>,
    stats_top_authors: Arc<StatsTopAuthorsRequestExecutor>,
    stats_top_posts: Arc<StatsTopPostsRequestExecutor>,
    stats_visits: Arc<StatsVisitsRequestExecutor>,
    subscribers: Arc<SubscribersRequestExecutor>,
    support_bots: Arc<SupportBotsRequestExecutor>,
    support_eligibility: Arc<SupportEligibilityRequestExecutor>,
    support_tickets: Arc<SupportTicketsRequestExecutor>,
}

impl WpComApiClient {
    pub fn new(delegate: WpApiClientDelegate) -> Self {
        let api_url_resolver: Arc<dyn ApiUrlResolver> =
            Arc::new(WpComApiClientInternalUrlResolver::default());

        api_client_generate_api_client!(
            api_url_resolver,
            delegate;
            followers,
            jetpack_connection,
            languages,
            me,
            oauth2,
            sites,
            stats_city_views,
            stats_clicks,
            stats_country_views,
            stats_devices_browser,
            stats_devices_platform,
            stats_devices_screensize,
            stats_referrers,
            stats_region_views,
            stats_top_authors,
            stats_top_posts,
            stats_visits,
            subscribers,
            support_bots,
            support_eligibility,
            support_tickets
        )
    }
}
api_client_generate_endpoint_impl!(WpComApi, followers);
api_client_generate_endpoint_impl!(WpComApi, jetpack_connection);
api_client_generate_endpoint_impl!(WpComApi, languages);
api_client_generate_endpoint_impl!(WpComApi, me);
api_client_generate_endpoint_impl!(WpComApi, oauth2);
api_client_generate_endpoint_impl!(WpComApi, sites);
api_client_generate_endpoint_impl!(WpComApi, stats_city_views);
api_client_generate_endpoint_impl!(WpComApi, stats_clicks);
api_client_generate_endpoint_impl!(WpComApi, stats_country_views);
api_client_generate_endpoint_impl!(WpComApi, stats_devices_browser);
api_client_generate_endpoint_impl!(WpComApi, stats_devices_platform);
api_client_generate_endpoint_impl!(WpComApi, stats_devices_screensize);
api_client_generate_endpoint_impl!(WpComApi, stats_referrers);
api_client_generate_endpoint_impl!(WpComApi, stats_region_views);
api_client_generate_endpoint_impl!(WpComApi, stats_top_authors);
api_client_generate_endpoint_impl!(WpComApi, stats_top_posts);
api_client_generate_endpoint_impl!(WpComApi, stats_visits);
api_client_generate_endpoint_impl!(WpComApi, subscribers);
api_client_generate_endpoint_impl!(WpComApi, support_bots);
api_client_generate_endpoint_impl!(WpComApi, support_eligibility);
api_client_generate_endpoint_impl!(WpComApi, support_tickets);
