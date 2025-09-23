use std::sync::Mutex;

#[derive(Debug, Default, uniffi::Object)]
pub struct RequestContext {
    request_ids: Mutex<Vec<String>>,
}

#[uniffi::export]
impl RequestContext {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            request_ids: Mutex::new(Vec::new()),
        }
    }

    pub fn add_request_id(&self, request_id: String) {
        if let Ok(mut ids) = self.request_ids.lock() {
            ids.push(request_id);
        }
    }

    pub fn request_ids(&self) -> Vec<String> {
        if let Ok(ids) = self.request_ids.lock() {
            return (*ids).clone();
        }

        vec![]
    }
}
