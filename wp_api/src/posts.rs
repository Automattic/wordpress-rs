pub struct PostRequestBuilder {}

impl PostRequestBuilder {
    pub fn list(&self, params: Option<PostListParams>) -> PostRequest {
        todo!()
    }

    pub fn create(&self, params: Option<PostCreateParams>) -> PostRequest {
        todo!()
    }

    pub fn retrieve(&self, post_id: u32, params: Option<PostRetrieveParams>) -> PostRequest {
        todo!()
    }

    pub fn update(&self, post_id: u32, params: Option<PostUpdateParams>) -> PostRequest {
        todo!()
    }

    pub fn delete(&self, post_id: u32, params: Option<PostDeleteParams>) -> PostRequest {
        todo!()
    }
}

pub struct PostListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct PostCreateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostRetrieveParams {
    pub password: Option<String>,
}

pub struct PostUpdateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostDeleteParams {
    pub force: Option<bool>,
}

pub struct PostRequest {
    pub endpoint: String,
    pub params: Option<String>,
}
