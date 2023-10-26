use config::{Config, File, FileFormat};
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
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    username: String,
    password: String,
    port: u16,
    host: String,
    db_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.db_name
        )
    }
}
pub fn get_config() -> Result<Settings, config::ConfigError> {
    let builder = Config::builder().add_source(File::new("config", FileFormat::Yaml));

    match builder.build() {
        Ok(config) => config.try_deserialize(),
        Err(e) => Err(e),
    }
}
