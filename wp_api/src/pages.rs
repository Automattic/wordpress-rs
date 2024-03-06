#[derive(uniffi::Record)]
pub struct PageListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(uniffi::Record, Debug)]
pub struct PageListResponse {
    pub page_list: Option<Vec<PageObject>>,
}

#[derive(uniffi::Record, Debug)]
pub struct PageObject {
    pub id: Option<u32>,
    pub title: Option<String>,
    pub content: Option<String>,
}
