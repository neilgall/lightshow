#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

mod controller;
mod iot_client;
mod settings;
mod zone;

use controller::Controller;
use settings::Settings;

fn main() {
	env_logger::init();

	let settings = Settings::new()
		.expect("Unable to load settings");
	debug!("settings: {:?}", settings);

	let mut controller = Controller::new(settings)
		.expect("Unable to initialise controller");

	controller.receive_loop();
}