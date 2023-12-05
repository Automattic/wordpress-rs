use wp_api::PostRequestBuilder;
use wp_networking::post_networking;
use wp_parsing::post_response_parser;

fn main() {
    println!("Hello, world!");
    let post_request_builder = PostRequestBuilder {};
    let post_list_request = post_request_builder.list(None);
    let post_list_response = post_networking().list(post_list_request);
    let post_list = post_response_parser().list(post_list_response);

    println!("Post List: {:?}", post_list);
}
