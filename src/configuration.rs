use secrecy::{ExposeSecret, Secret};
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::domain::SubscriberEmail;

#[derive(Clone, Debug, PartialEq)]
pub enum Enviroment {
    Local,
    Production,
}

impl Enviroment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Enviroment::Local => "local",
            Enviroment::Production => "production",
        }
    }
}

impl TryFrom<String> for Enviroment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not supported enviroment. use either local or enviroment",
                other
            )),
        }
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Clone, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(Clone, serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut setting = config::Config::default();

    let base_path = std::env::current_dir().expect("Failed to determinate current directory");
    let configuration_directory = base_path.join("configuration");

    setting.merge(config::File::from(configuration_directory.join("base")).required(true))?;

    let enviroment: Enviroment = std::env::var("APP_ENVIROMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIROMENT");

    setting.merge(
        config::File::from(configuration_directory.join(enviroment.as_str())).required(true),
    )?;

    setting.try_into()
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let _ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

impl Settings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.host,
            self.database.username,
            self.database.password.expose_secret(),
            self.database.port,
            self.database.database_name
        ))
    }
    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.database.host,
            self.database.username,
            self.database.password.expose_secret(),
            self.database.port
        ))
    }
}
