use linkify::{LinkFinder, LinkKind};
use rocket::{
    form::Form,
    fs::{FileServer, relative},
};
use rocket_dyn_templates::{Template, context};
use std::sync::Arc;
use wp_api::{
    login::login_client::WpLoginClient, reqwest_request_executor::ReqwestRequestExecutor,
};

#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

#[derive(FromForm)]
struct TestForm<'r> {
    value: &'r str,
}

#[post("/test", data = "<form>")]
async fn test(form: Form<TestForm<'_>>) -> Template {
    let request_executor = Arc::new(ReqwestRequestExecutor::default());
    let login_client = WpLoginClient::new_with_default_middleware_pipeline(request_executor);

    println!("Testing {}", form.value);

    match login_client
        .api_discovery(form.value.to_string())
        .await
        .combined_result()
    {
        Ok(success) => {
            let auth_url = success
                .api_details
                .find_application_passwords_authentication_url()
                .expect("Already confirmed auto discovery was successful");

            Template::render(
                "results",
                context! {
                    value: form.value,
                    application_passwords_authentication_url: linkify_text(&auth_url, false),
                    is_error: false
                },
            )
        }
        Err(error) => Template::render(
            "results",
            context! {
                value: form.value,
                error: linkify_text(&error.to_string(), false),
                is_error: true
            },
        ),
    }
}

pub fn linkify_text(text: &str, allow_without_scheme: bool) -> String {
    let mut link_finder = LinkFinder::new();
    link_finder.url_must_have_scheme(!allow_without_scheme);
    let mut bytes = Vec::new();
    for span in link_finder.spans(text) {
        match span.kind() {
            Some(LinkKind::Url) => {
                let mut url = span.as_str().to_string();
                if !url.contains(":") {
                    url.insert_str(0, "https://");
                }
                bytes.extend_from_slice(b"<a href=\"");
                escape(url.trim(), &mut bytes);
                bytes.extend_from_slice(b"\" title=\"URL\">");
                escape(span.as_str(), &mut bytes);
                bytes.extend_from_slice(b"</a>");
            }
            Some(LinkKind::Email) => {
                bytes.extend_from_slice(b"<a href=\"mailto:");
                escape(span.as_str(), &mut bytes);
                bytes.extend_from_slice(b"\" title=\"email\">");
                escape(span.as_str(), &mut bytes);
                bytes.extend_from_slice(b"</a>");
            }
            _ => {
                escape(span.as_str(), &mut bytes);
            }
        }
    }
    String::from_utf8(bytes).expect("added bytes are all ASCII")
}

fn escape(text: &str, dest: &mut Vec<u8>) {
    for c in text.bytes() {
        match c {
            b'&' => dest.extend_from_slice(b"&amp;"),
            b'<' => dest.extend_from_slice(b"&lt;"),
            b'>' => dest.extend_from_slice(b"&gt;"),
            b'"' => dest.extend_from_slice(b"&quot;"),
            b'\'' => dest.extend_from_slice(b"&#39;"),
            _ => dest.push(c),
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, test])
        .mount("/assets", FileServer::from(relative!("assets")))
        .attach(Template::fairing())
}
