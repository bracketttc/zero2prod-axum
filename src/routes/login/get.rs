use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_flash::IncomingFlashes;
use std::fmt::Write;

pub async fn login_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let mut error_html = String::new();
    for (_, text) in flashes
        .iter()
        .filter(|(level, _)| level == &axum_flash::Level::Error)
    {
        writeln!(error_html, "<p><i>{text}</i></p>").unwrap();
    }

    (
        StatusCode::OK,
        flashes,
        Html(format!(
            r#"<!DOCTYPE html>
        <html lang="en">
        
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8">
            <title>Login</title>
        </head>
        
        <body>
            {error_html}
            <form action="/login" method="post">
                <label>Username
                    <input type="text" placeholder="Enter Username" name="username">
                </label>
        
                <label>Password
                    <input type="password" placeholder="Enter Password" name="password">
                </label>
        
                <button type="submit">Login</button>
            </form>
        </body>
        
        </html>"#,
        )),
    )
}
