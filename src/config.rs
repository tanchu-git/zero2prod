use config::{Config, File, FileFormat};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    database: DatabaseSettings,
    application_port: u16,
}

impl Settings {
    pub fn get_port(&self) -> u16 {
        self.application_port
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
pub fn get_config() -> Result<Settings, config::ConfigError> {
    let builder = Config::builder().add_source(File::new("config", FileFormat::Yaml));

    match builder.build() {
        Ok(config) => config.try_deserialize(),
        Err(e) => Err(e),
    }
}
