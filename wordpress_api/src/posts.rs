pub struct PostsRequestBuilder {}

impl PostsRequestBuilder {
    pub fn list(&self, params: Option<PostsListParams>) -> PostsRequest {
        todo!()
    }

    pub fn create(&self, params: Option<PostsCreateParams>) -> PostsRequest {
        todo!()
    }

    pub fn retrieve(&self, post_id: u32, params: Option<PostsRetrieveParams>) -> PostsRequest {
        todo!()
    }

    pub fn update(&self, post_id: u32, params: Option<PostsUpdateParams>) -> PostsRequest {
        todo!()
    }

    pub fn delete(&self, post_id: u32, params: Option<PostsDeleteParams>) -> PostsRequest {
        todo!()
    }
}

pub struct PostsListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub struct PostsCreateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostsRetrieveParams {
    pub password: Option<String>,
}

pub struct PostsUpdateParams {
    pub title: Option<String>,
    pub content: Option<String>,
}

pub struct PostsDeleteParams {
    pub force: Option<bool>,
}

pub struct PostsRequest {
    pub endpoint: String,
    pub params: Option<String>,
}

uniffi::include_scaffolding!("posts");
