use askama::Template;
use axum::response::{Html, IntoResponse, Response};
use axum_flash::IncomingFlashes;

#[derive(Template)]
#[template(path = "admin/newsletter.html")]
struct NewsletterForm<'a> {
    flashes: &'a IncomingFlashes,
}

pub async fn newsletter_form(flashes: IncomingFlashes) -> Response {
    let page = NewsletterForm { flashes: &flashes }.render().unwrap();
    (flashes, Html(page)).into_response()
}
