use once_cell::sync::Lazy;
use reqwest::Request;
use secrecy::ExposeSecret;
use sqlx::Executor;
use sqlx::{Connection, PgConnection, PgPool};
use std::io::{sink, stdout};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, sink);
        init_subscriber(subscriber);
    }
});

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub database_name: String,
    pub db_configuration: DatabaseSettings
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new().links(s).filter(|l| *l.kind() == linkify::LinkKind::Url).collect();

            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();

            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();

            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Obtenemos los datos necesarios
        let db_name = self.database_name.clone();
        let config = self.db_configuration.clone();

        // Ejecutamos la limpieza en un hilo separado para manejar el async
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // 1. Conectar a la base de datos por defecto ('postgres') para poder borrar la otra
                let mut options = sqlx::postgres::PgConnectOptions::new()
                    .host(&config.host)
                    .port(config.port)
                    .username(&config.username)
                    .password(config.password.expose_secret());
                
                let mut connection = PgConnection::connect_with(&options)
                    .await
                    .expect("Failed to connect to Postgres for cleanup");

                // 2. Forzar el cierre de conexiones activas (importante: sqlx mantiene pools abiertos)
                let terminate_query = format!(
                    r#"SELECT pg_terminate_backend(pid) 
                       FROM pg_stat_activity 
                       WHERE datname = '{}' AND pid <> pg_backend_pid()"#,
                    db_name
                );
                let _ = connection.execute(terminate_query.as_str()).await;

                // 3. Borrar la base de datos
                let drop_query = format!(r#"DROP DATABASE "{}""#, db_name);
                let _ = connection.execute(drop_query.as_str()).await;
                
                println!("Successfully dropped database: {}", db_name);
            });
        })
        .join()
        .expect("The cleanup thread panicked");
    }
}

// launch de app in the background
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("failed build application");

    let application_port = application.port();

    let address = format!("http://127.0.0.1:{}", application_port);

    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port,
        database_name: configuration.database.database_name.clone(),
        db_configuration: configuration.database.clone()
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create DB");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connecto to postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database");

    connection_pool
}
