extern crate ble_advert_struct;
#[macro_use]
extern crate derive_more;
extern crate rumqtt;
extern crate serde_json;

use ble_advert_struct::BLEAdvert;
use rumqtt::{MqttClient, MqttOptions, Notification, Publish, QoS};
use std::env;
use std::str::from_utf8;

const PKG_NAME: &'static str = env!("CARGO_PKG_NAME");

fn get_env(s: &str) -> String{
    env::var(s).map_err(|e| format!("Error reading environment variable {}: {}", s, e)).unwrap()
}

#[derive(Debug, From)]
enum Error {
    Utf8Error(std::str::Utf8Error),
    JsonError(serde_json::Error),
}

fn decode_advert(p: &Publish) -> Result<BLEAdvert, Error> {
    let s = from_utf8(&p.payload)?;
    let advert: BLEAdvert = serde_json::from_str::<BLEAdvert>(s)?;
    Ok(advert)
}

fn main() {
    let mqtt_host = get_env("MQTT_HOST");
    let mqtt_topic = get_env("MQTT_TOPIC");

    let mqtt_options = MqttOptions::new(PKG_NAME, mqtt_host, 1883);
    let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options).unwrap();

    mqtt_client.subscribe(mqtt_topic, QoS::AtLeastOnce).unwrap();

    for notification in notifications {
        if let Notification::Publish(p) = notification {
            let advert = match decode_advert(&p) {
                Ok(advert) => advert,
                Err(error) => {
                    println!("{:?}", error);
                    continue;
                },
            };
            println!("{:?}", advert);
            // TODO: decode
            // TODO: punt to influxdb
        }
    }
}
