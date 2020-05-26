# Lightshow

I wrote this to control the lights in our summerhouse and garden
using AWS IoT device shadows, which can be updated via a lambda
function and hence by Alexa.

In reality it's a generic IoT shadow to Raspberry Pi GPIO pin
controller. It could be used to turn anything on and off this way.

## Overview

`iot_client.rs` uses the [rumqtt](https://docs.rs/rumqtt/0.31.0/rumqtt/) MQTT client to connect directly to [AWS IoT core](https://docs.aws.amazon.com/iot/index.html) via a TLS connection. This requires a private key and certificate to be generated in IoT core, downloaded and referenced in the `iot_client` section of the `Settings.yml` file. The client only subscribes to shadow `update/delta` and `get/accepted` topics. 

`zone.rs` sets up [rustpi_io](https://skasselbard.github.io/rustpiIO/doc/rustpi_io/index.html) GPIOs for each of the zones defined in `Settings.yml`. Each zone has a list of pins which are activated with a configurable delay. My garden system is solar powered and the solar controller doesn't like the startup current of all the LED lights coming on at once.

`settings.rs` uses the [config](https://docs.rs/config/0.10.1/config/) crate to read the `Settings.yml` file into Rust structs.

`controller.rs` links the IoT client with the zones. On startup it publishes to the `get` MQTT topic via the IoT client, then enters a receive loop. When `get/accepted` and `update/delta` events arrive the appropriate zones are configured. If a zone changes state, a new reported state is published back to IoT core.

The IoT core client monitors disconnect and reconnect events. After the second of these occurs, it notifies the controller which re-registers the MQTT subscriptions. Since my Raspberry Pi where this runs is several metres from the house its WiFi connection is not perfectly stable, and I have found this resubscription absolutely vital for reliable operation of MQTT.
