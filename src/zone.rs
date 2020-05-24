use rustpi_io::gpio::*;
use std::error::Error;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct ZoneConfig {
	pub name: String,
	pub device_name: String,
	pub pins: Vec<u8>,
	pub delay_millis: u64
}


pub struct Zone {
	name: String,
	gpios: Vec<GPIO>,
	delay: Duration
}


impl Zone {
	pub fn new(config: &ZoneConfig) -> Self{
		let mut gpios = vec![];
		for pin in config.pins.iter() {
			if let Ok(gpio) = GPIO::new(*pin, GPIOMode::Write) {
				gpios.push(gpio);
			} else {
				error!("can't initialise GPIO pin {}", pin);
			}
		}
		Zone { 
			name: String::from(&config.name),
			gpios,
			delay: Duration::from_millis(config.delay_millis)
		}
	}

	pub fn set_state(&self, state: bool) -> Result<(), Box<dyn Error>> {
		debug!("zone {} state {}", self.name, state);
		let value = if state { GPIOData::High } else { GPIOData::Low };
		for pin in &self.gpios {
			pin.set(value)?;
			thread::sleep(self.delay);
		}
		Ok(())
	}
}