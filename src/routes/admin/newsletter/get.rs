use askama::Template;
use axum::response::{Html, IntoResponse, Response};
use axum_flash::IncomingFlashes;

#[derive(Template)]
#[template(path = "admin/newsletter.html")]
struct NewsletterForm<'a> {
    flashes: &'a IncomingFlashes,
    idempotency_key: uuid::Uuid,
}

pub async fn newsletter_form(flashes: IncomingFlashes) -> Response {
    let page = NewsletterForm {
        flashes: &flashes,
        idempotency_key: uuid::Uuid::new_v4(),
    }
    .render()
    .unwrap();
    (flashes, Html(page)).into_response()
}
