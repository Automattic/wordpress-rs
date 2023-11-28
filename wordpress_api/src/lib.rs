#![allow(dead_code)]
use std::sync::Arc;

use posts::*;

mod posts;

pub fn add_custom(left: i32, right: i32) -> i32 {
    left + right
}

pub fn combine_strings(a: String, b: String) -> String {
    format!("{}-{}", a, b)
}

pub fn panic_from_rust() {
    std::fs::read_to_string("doesnt_exist.txt").unwrap();
}

struct RequestBuilder {}

impl RequestBuilder {
    fn posts(&self) -> Arc<PostsRequestBuilder> {
        Arc::new(PostsRequestBuilder {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add_custom(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_combine_strings() {
        let result = combine_strings("this".into(), "that".into());
        assert_eq!(result, "this-that");
    }

    #[test]
    #[should_panic]
    fn test_panic_from_rust() {
        panic_from_rust()
    }
}

uniffi::include_scaffolding!("wordpress_api");
