use crate::{
    request::endpoint::{AsNamespace, DerivedRequest},
    wp_com::{
        WpComNamespace, WpComSiteId,
        subscribers::{
            AddSubscribersParams, AddSubscribersResponse, IndividualSubscriberParams,
            IndividualSubscriberStats, IndividualSubscriberStatsParams, ListSubscribersResponse,
            Subscriber, SubscriberImportJob, SubscriberImportJobsListParams,
            SubscriberStatsResponse, SubscribersByUserTypeParams, SubscribersListParams, UploadId,
        },
    },
};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum SubscribersRequest {
    #[get(url = "/sites/<wp_com_site_id>/subscribers", params = &SubscribersListParams, output = ListSubscribersResponse)]
    ListSubscribers,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/individual", params = &IndividualSubscriberParams, output = Subscriber)]
    IndividualSubscriber,
    #[get(url = "/sites/<wp_com_site_id>/individual-subscriber-stats", params = &IndividualSubscriberStatsParams, output = IndividualSubscriberStats)]
    IndividualSubscriberStats,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/import", params = &SubscriberImportJobsListParams, output = Vec<SubscriberImportJob>)]
    ListSubscriberImportJobs,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/import/<upload_id>", output = SubscriberImportJob)]
    GetSubscriberImportJob,
    #[post(url = "/sites/<wp_com_site_id>/subscribers/import", params = &AddSubscribersParams, output = AddSubscribersResponse)]
    AddSubscribers,
    #[get(url = "/sites/<wp_com_site_id>/subscribers/stats", output = SubscriberStatsResponse)]
    GetSubscriberStats,
    #[get(url = "/sites/<wp_com_site_id>/subscribers_by_user_type", params = &SubscribersByUserTypeParams, output = ListSubscribersResponse)]
    ListSubscribersByUserType,
}

impl DerivedRequest for SubscribersRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpComNamespace::V2
    }
}
