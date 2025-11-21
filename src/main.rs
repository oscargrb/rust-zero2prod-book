use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::{io::stdout, net::TcpListener};
use zero2prod::{
    configuration::get_configuration,
    email_client::EmailClient,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Log's configuration
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), stdout);

    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(&configuration.database.connection_string().expose_secret())
        .expect("Error");

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");

    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let listener = TcpListener::bind(address)?;

    println!(
        "server running on {}",
        listener.local_addr().expect("Error")
    );

    let _ = run(listener, connection_pool, email_client)
        .await
        .expect("Error");

    Ok(())
}
