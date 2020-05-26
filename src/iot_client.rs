
use rumqtt::{QoS, ReconnectOptions, Receiver, MqttClient, MqttOptions};
use regex::Regex;
use serde_json;
use std::fs::read;
use std::sync::{Arc, mpsc::Sender};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

pub use rumqtt::{ClientError, ConnectError};

#[derive(Debug, Deserialize)]
pub struct IoTClientConfig {
    pub client_id: String,
    pub host: String,
    pub port: u16,
    pub root_ca_path: String,
    pub private_key_path: String,
    pub certificate_path: String    
}


pub struct IoTClient {
    mqtt_client: MqttClient,
    thread: Option<JoinHandle<()>>,
}

impl Drop for IoTClient {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[derive(Debug)]
pub enum IoTEvent {
    Reconnect,

    Get {
        thing_name: String,
        shadow: serde_json::Value
    },

    Delta {
        thing_name: String,
        shadow: serde_json::Value
    }
}

fn decode_topic_name(topic_name: &str) -> Option<(String, String)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\$aws/things/([^/]+)/shadow/([^/]+)/").unwrap();
    }
    let captures = RE.captures(topic_name)?;
    Some( (String::from(captures.get(1)?.as_str()), String::from(captures.get(2)?.as_str()) ) )
}

fn decode_payload(payload: &Arc<Vec<u8>>) -> Option<serde_json::Value> {
    let payload_str = String::from_utf8(payload.to_vec()).ok()?;
    Some(serde_json::from_str(&payload_str).ok()?)
}

fn send(sender: &Sender<IoTEvent>, event: IoTEvent) {
    match sender.send(event) {
        Ok(_) => {}
        Err(e) => error!("Failed to send event: {:?}", e)
    }
}

fn main(receiver: Receiver<rumqtt::client::Notification>, sender: Sender<IoTEvent>) {
    debug!("iot client thread started");
    let mut disconnected = false;
    loop {
        for notification in &receiver {
            match &notification {
                rumqtt::client::Notification::Disconnection => {
                    debug!("notification: Disconnection");
                    disconnected = true;
                }

                rumqtt::client::Notification::Reconnection if disconnected => {
                    debug!("notification: Reconnection");
                    send(&sender, IoTEvent::Reconnect);
                    disconnected = false;
                }

                rumqtt::client::Notification::Publish(packet) => {
                    debug!("notification: Publish {}", packet.topic_name);

                    if let Some( (thing_name, action) ) = decode_topic_name(&packet.topic_name) {
                        if let Some(shadow) = decode_payload(&packet.payload) {
                            if action == "get" {
                                send(&sender, IoTEvent::Get { thing_name, shadow });
                            } else if action == "update" {
                                send(&sender, IoTEvent::Delta { thing_name, shadow });
                            } else {
                                error!("Failed to decode notification {:?}", notification);
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

impl IoTClient {
    pub fn new(config: &IoTClientConfig, sender: Sender<IoTEvent>) -> Result<Self, ConnectError> {
        let mqtt_options = MqttOptions::new(&config.client_id, &config.host, config.port)
            .set_ca(read(&config.root_ca_path)?)
            .set_client_auth(read(&config.certificate_path)?, read(&config.private_key_path)?)
            .set_keep_alive(10)
            .set_reconnect_opts(ReconnectOptions::Always(5));
        let (mqtt_client, receiver) = MqttClient::start(mqtt_options)?;
        
        let thread = spawn(move || { main(receiver, sender) });

        Ok(IoTClient {
            mqtt_client,
            thread: Some(thread)
        })
    }

    pub fn get_shadow(&mut self, thing_name: &str) -> Result<(), ClientError> {
        let shadow_get_accepted = format!("$aws/things/{}/shadow/get/accepted", thing_name);
        self.mqtt_client.subscribe(&shadow_get_accepted, QoS::AtMostOnce)?;
        sleep(Duration::from_millis(250));

        let shadow_get = format!("$aws/things/{}/shadow/get", thing_name);
        self.mqtt_client.publish(shadow_get, QoS::AtMostOnce, false, "{}")
    }

    pub fn subscribe_to_shadow_delta(&mut self, thing_name: &str) -> Result<(), ClientError> {
        let shadow_topic = String::from(format!("$aws/things/{}/shadow/update/delta", thing_name));
        self.mqtt_client.subscribe(&shadow_topic, QoS::AtMostOnce)
    }

    pub fn publish_shadow(&mut self, thing_name: &str, shadow: serde_json::Value) -> Result<(), ClientError> {
        let shadow_topic = String::from(format!("$aws/things/{}/shadow/update", thing_name));
        self.mqtt_client.publish(shadow_topic, QoS::AtMostOnce, false, shadow.to_string())
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_topic_name_get() {
        let decode = decode_topic_name(&"$aws/things/foo-device/shadow/get/accepted").unwrap();
        assert_eq!(String::from("foo-device"), decode.0);
        assert_eq!(String::from("get"), decode.1);
    }

    #[test]
    fn test_decode_topic_name_update() {
        let decode = decode_topic_name("$aws/things/bar-device/shadow/update/delta").unwrap();
        assert_eq!(String::from("bar-device"), decode.0);
        assert_eq!(String::from("update"), decode.1);
    }

    #[test]
    fn test_decode_topic_name_missing_thing_name() {
        assert!(decode_topic_name("$aws/things/shadow/get/accepted").is_none());
    }
}
