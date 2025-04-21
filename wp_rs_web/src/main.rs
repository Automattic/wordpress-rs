use std::sync::Arc;

use linkify::{LinkFinder, LinkKind};
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::fs::relative;
use rocket_dyn_templates::{Template, context};
use wp_api::login::login_client::WpLoginClient;
use wp_api::login::url_discovery::AutoDiscoveryAttemptType;
use wp_api::middleware::WpApiMiddlewarePipeline;
use wp_api::reqwest_request_executor::ReqwestRequestExecutor;

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
    let request_executor = Arc::new(ReqwestRequestExecutor::new_with_default_timeout(false));
    let login_client = WpLoginClient::new(
        request_executor,
        Arc::new(WpApiMiddlewarePipeline {
            middlewares: vec![],
        }),
    );

    println!("Testing {}", form.value);
    let result: wp_api::login::url_discovery::AutoDiscoveryResult =
        login_client.api_discovery(form.value.to_string()).await;

    if result.is_successful() {
        let attempt = result.find_successful().unwrap();

        let application_passwords_authentication_url = attempt
            .api_discovery_result
            .clone()
            .unwrap()
            .api_details
            .find_application_passwords_authentication_url();

        Template::render(
            "results",
            context! {
                value: form.value,
                result: result.is_successful().to_string(),
                application_passwords_authentication_url: application_passwords_authentication_url,
                is_error: false
            },
        )
    } else {
        if let Some(attempt) = result
            .attempts
            .get(&AutoDiscoveryAttemptType::AutoStrippedHttps)
        {
            if let Some(error) = attempt.api_discovery_result.as_ref().err() {
                return Template::render(
                    "results",
                    context! {
                        value: form.value,
                        error: linkify_text(&error.to_string(), false),
                        is_error: true
                    },
                );
            }
        }

        let attempt = result.user_input_attempt();
        if let Some(error) = attempt.api_discovery_result.as_ref().err() {
            Template::render(
                "results",
                context! {
                    value: form.value,
                    error: linkify_text(&error.to_string(), false),
                    is_error: true
                },
            )
        } else {
            Template::render(
                "results",
                context! {
                    value: form.value,
                    error: linkify_text("Unknown error", false),
                    is_error: true
                },
            )
        }
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
