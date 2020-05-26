use std::collections::HashMap;
use std::sync::mpsc;

use crate::iot_client::{IoTClient, IoTEvent, ConnectError, ClientError};
use crate::settings::Settings;
use crate::zone::Zone;

#[derive(Debug)]
enum UpdateError {
	ShadowError,
	UnknownZone(String),
	GPIOError(std::io::Error),
	IoTError(ClientError)
}

pub struct Controller {
	iot_client: IoTClient,
	rx: mpsc::Receiver<IoTEvent>,
	zones: HashMap<String, Zone>
}

impl Controller {
	pub fn new(config: Settings) -> Result<Self, ConnectError> {
		let mut zones = HashMap::new();
		for zone_config in config.zones.iter() {
			let zone = Zone::new(zone_config);
			zones.insert(String::from(&zone_config.device_name), zone);
		}

		let (tx, rx) = mpsc::channel();

		let iot_client = IoTClient::new(&config.iot_client, tx)?;

		Ok(Controller {
			iot_client,
			rx,
			zones
		})
	}

	pub fn receive_loop(&mut self) {
		self.register_devices();
		loop {
			let event = self.rx.recv().expect("receive error");
			match event {
				IoTEvent::Reconnect => {
					// re-register subscriptions on reconnection
					self.register_devices();
				}

				IoTEvent::Get { thing_name, shadow } => {
					match self.handle_get(&thing_name, &shadow) {
						Ok(_) => {}
						Err(e) => error!("Failed to handle get response {}: {:?}", thing_name, e)
					} 
				}

				IoTEvent::Delta { thing_name, shadow } => {
					match self.handle_delta(&thing_name, &shadow) {
						Ok(_) => {}
						Err(e) => error!("Failed to handle get delta {}: {:?}", thing_name, e)
					} 
				}
			}
		}
	}

	fn register_devices(&mut self) {
		for zone in self.zones.values() {
			match self.iot_client.get_shadow(&zone.device_name) {
				Ok(_) => {}
				Err(e) => error!("Unable to get shadow for {}: {}", zone.name, e)
			}
			match self.iot_client.subscribe_to_shadow_delta(&zone.device_name) {
				Ok(_) => {}
				Err(e) => error!("Unable to subscribe to shadow delta for {}: {}", zone.name, e)
			}
		}
	}

	fn handle_get(&mut self, thing_name: &String, shadow: &serde_json::Value) -> Result<(), UpdateError> {
		fn parse(shadow: &serde_json::Value) -> Option<String> {
			let state = shadow["state"]["desired"]["state"].as_str()?;
			Some(String::from(state))
		}
		debug!("Receive get response {}, {}", thing_name, shadow.to_string());
		let state = parse(shadow).ok_or(UpdateError::ShadowError)?;
		let zone = self.zones.get_mut(thing_name).ok_or(UpdateError::UnknownZone(thing_name.to_string()))?;
		zone.set_state(state == "ON").map_err(UpdateError::GPIOError)?;

		// always publish reported state after a get
		self.publish_new_state(thing_name, &state)
	}

	fn handle_delta(&mut self, thing_name: &String, shadow: &serde_json::Value) -> Result<(), UpdateError> {
		fn parse(shadow: &serde_json::Value) -> Option<String> {
			let state = shadow["state"]["state"].as_str()?;
			Some(String::from(state))
		}
		debug!("Receive shadow update {}, {}", thing_name, shadow.to_string());
		let state = parse(shadow).ok_or(UpdateError::ShadowError)?;
		let zone = self.zones.get_mut(thing_name).ok_or(UpdateError::UnknownZone(thing_name.to_string()))?;
		let updated = zone.set_state(state == "ON").map_err(UpdateError::GPIOError)?;

		// publish new state if a delta event results in a change
		if !updated {
			debug!("No change");
			Ok(())
		} else {
			self.publish_new_state(thing_name, &state)
		}
	}

	fn publish_new_state(&mut self, thing_name: &String, state: &String) -> Result<(), UpdateError> {
		let new_shadow = json!({
			"state": {
				"reported": { 
					"state": state
				}
			}
		});
		debug!("Publish shadow update {}", new_shadow.to_string());
		self.iot_client.publish_shadow(&thing_name, new_shadow).map_err(UpdateError::IoTError)		
	}
}
