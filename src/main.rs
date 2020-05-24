#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use std::collections::HashMap;
use std::sync::mpsc;

mod iot_client;
mod settings;
mod zone;

use iot_client::{IoTClient, IoTEvent};
use settings::Settings;
use zone::Zone;


fn register_devices<'a>(iot_client: &mut IoTClient, zones: impl Iterator<Item = &'a Zone>) {
	for zone in zones {
		iot_client.get_shadow(&zone.device_name);
		iot_client.add_listen_on_delta_callback(&zone.device_name);
	}
}

fn handle_get(thing_name: &String, shadow: &serde_json::Value, zones: &HashMap<String, Zone>) -> Option<()> {
	debug!("get {}, {:?}", thing_name, shadow);
	let state = shadow.get("state")?.get("desired")?.get("state")?.as_str()? == "ON";
	let zone = zones.get(thing_name)?;
	zone.set_state(state).ok()?;
	Some(())
}

fn handle_update(thing_name: &String, shadow: &serde_json::Value, zones: &HashMap<String, Zone>) -> Option<()> {
	debug!("update {}, {:?}", thing_name, shadow);
	let state = shadow.get("state")?.get("desired")?.get("state")?.as_str()? == "ON";
	let zone = zones.get(thing_name)?;
	zone.set_state(state).ok()?;
	Some(())
}

fn receive_loop(mut iot_client: &mut IoTClient, rx: &mpsc::Receiver<IoTEvent>, zones: &HashMap<String, Zone>) {
	register_devices(&mut iot_client, zones.values());
	loop {
		let event = rx.recv().expect("receive error");
		match event {
			IoTEvent::Reconnect => {
				// re-register subscriptions on reconnection
				register_devices(&mut iot_client, zones.values());
			}

			IoTEvent::Get { thing_name, shadow } => {
				if handle_get(&thing_name, &shadow, &zones).is_none() {
					error!("failed to handle get {} {:?}", thing_name, shadow);
				}
			}

			IoTEvent::Update { thing_name, shadow } => {
				if handle_update(&thing_name, &shadow, &zones).is_none() {
					error!("failed to handle update {} {:?}", thing_name, shadow);
				}
			}
		}
	}
}


fn main() {
	env_logger::init();

	let s = Settings::new()
		.expect("Unable to load settings");

	debug!("settings: {:?}", s);

	let (tx, rx) = mpsc::channel();

	let mut zones = HashMap::new();
	for zone_config in s.zones.iter() {
		let zone = Zone::new(zone_config);
		zones.insert(String::from(&zone_config.device_name), zone);
	}

	loop {
		let mut iot_client = IoTClient::new(&s.iot_client, tx.clone())
			.expect("Unable to initialise IoT client");

		receive_loop(&mut iot_client, &rx, &zones);
	}
}