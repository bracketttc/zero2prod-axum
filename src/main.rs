use zero2prod_axum::configuration::get_configuration;
use zero2prod_axum::startup::build;
use zero2prod_axum::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = get_subscriber("zero2prod_axum".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    build(configuration).await?;
    Ok(())
}
