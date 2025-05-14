use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        subscribers::{
            AddSubscribersParams, AddSubscribersResponse, GetSubscriberQuery,
            ListSubscriberImportJobsParams, ListSubscribersParams, ListSubscribersResponse,
            SubscriberImportJob, SubscriberStatsResponse, UploadId,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SubscribersRequest {
    #[get(url = "/sites/<wp_com_site_id>/subscribers", params = &ListSubscribersParams, output = ListSubscribersResponse)]
    ListSubscribers,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/individual", params = &GetSubscriberQuery, output = SubscriberImportJob)]
    GetSubscriber,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/import", params = &ListSubscriberImportJobsParams, output = Vec<SubscriberImportJob>)]
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
