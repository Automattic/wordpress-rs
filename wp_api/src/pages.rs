pub struct PageRequestBuilder {}

impl PageRequestBuilder {
    pub fn list(&self, params: Option<PageListParams>) -> PageRequest {
        todo!()
    }
}

pub struct PageListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct PageRequest {
    pub endpoint: String,
    pub params: Option<String>,
}
