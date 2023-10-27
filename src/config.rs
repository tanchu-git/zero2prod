use config::{Config, File};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    database: DatabaseSettings,
    app: AppSettings,
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
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    username: String,
    password: Secret<String>,
    port: u16,
    host: String,
    db_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.db_name
        ))
    }

    pub fn connection_string_no_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}

#[derive(Deserialize)]
pub struct AppSettings {
    port: u16,
    host: String,
}

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

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

    // Read the "default" configuration file
    let builder =
        Config::builder().add_source(File::from(config_directory.join("base")).required(true));

    // Detect the running environment.
    // Default to `local` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .unwrap();

    // Layer on the environment-specific values.
    let final_builder = builder
        .clone()
        .add_source(File::from(config_directory.join(environment.as_str())).required(true));

    match final_builder.build() {
        Ok(config) => config.try_deserialize(),
        Err(e) => Err(e),
    }
}
