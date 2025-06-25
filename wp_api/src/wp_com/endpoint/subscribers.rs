use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        subscribers::{
            AddSubscribersParams, AddSubscribersResponse, GetSubscriberQuery,
            ListSubscribersResponse, Subscriber, SubscriberImportJob,
            SubscriberImportJobsListParams, SubscriberStatsResponse, SubscribersListParams,
            UploadId,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SubscribersRequest {
    #[get(url = "/sites/<wp_com_site_id>/subscribers", params = &SubscribersListParams, output = ListSubscribersResponse)]
    ListSubscribers,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/individual", params = &GetSubscriberQuery, output = Subscriber)]
    GetSubscriber,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/import", params = &SubscriberImportJobsListParams, output = Vec<SubscriberImportJob>)]
    ListSubscriberImportJobs,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/import/<upload_id>", output = SubscriberImportJob)]
    GetSubscriberImportJob,
    #[post(url = "/sites/<wp_com_site_id>/subscribers/import", params = &AddSubscribersParams, output = AddSubscribersResponse)]
    AddSubscribers,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/stats", output = SubscriberStatsResponse)]
    GetSubscriberStats,
}

impl DerivedRequest for SubscribersRequest {
    fn namespace() -> impl AsNamespace {
        WpComNamespace::V2
    }
}
