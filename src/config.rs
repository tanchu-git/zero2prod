use config::{Config, File};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_aux::prelude::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;

use crate::domain::subscriber_email::SubscriberEmail;

#[derive(Deserialize)]
pub struct Settings {
    database: DatabaseSettings,
    app: AppSettings,
    email_client: EmailClientSettings,
}

impl Settings {
    pub fn get_app_port(&self) -> u16 {
        self.app.port
    }

    pub fn get_app_host(&self) -> &str {
        &self.app.host
    }

    pub fn get_db(&self) -> &DatabaseSettings {
        &self.database
    }

    pub fn set_db_name(&mut self, db_name: String) {
        self.database.db_name = db_name;
    }

    pub fn get_db_name(&self) -> &str {
        &self.database.db_name
    }

    pub fn get_email_client(&self) -> &EmailClientSettings {
        &self.email_client
    }
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    username: String,
    password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
    host: String,
    db_name: String,
    require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        let options = self.without_db().database(&self.db_name);

        options.log_statements(tracing::log::LevelFilter::Trace)
    }
}

#[derive(Deserialize)]
pub struct AppSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
    host: String,
}

#[derive(Deserialize)]
pub struct EmailClientSettings {
    base_url: String,
    sender_email: String,
    secret: Secret<String>,
    timeout_ms: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }

    pub fn get_secret(&self) -> Secret<String> {
        self.secret.clone()
    }

    pub fn get_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }
}

// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

// Convert from env string to enum Environment 'try_into()'
impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}

pub fn get_config() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory.");

    let config_directory = base_path.join("config");

    // Detect the running environment
    // Default to `local` if unspecified
    // Perform the conversion string -> Environment
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    let environment_filename = format!("{}.yaml", environment.as_str());

    // Read the base config settings
    // Layer on the environment-specific ('local' or 'production') values.
    let settings = Config::builder()
        .add_source(File::from(config_directory.join("base")).required(true))
        .add_source(config::File::from(
            config_directory.join(environment_filename),
        ))
        // Add any env variables with a prefix of APP and '__' as seperator
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
