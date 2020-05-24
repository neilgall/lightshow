extern crate config;
extern crate serde;

use config::{Config, ConfigError, File, FileFormat};

use crate::iot_client::IoTClientConfig;
use crate::zone::ZoneConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub iot_client: IoTClientConfig,
	pub zones: Vec<ZoneConfig>,
}

impl Settings {
	pub fn new() -> Result<Self, ConfigError> {
		let mut config = Config::new();
		config.merge(File::new("Settings", FileFormat::Yaml))?;
		config.try_into()
	}
}
