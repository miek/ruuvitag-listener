extern crate ble_advert_struct;
extern crate byteorder;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate influx_db_client;
extern crate rumqtt;
extern crate serde_json;

mod ruuvipacket;

use ble_advert_struct::BLEAdvert;
use rumqtt::{MqttClient, MqttOptions, Notification, Publish, QoS};
use std::env;
use std::io::Cursor;
use std::str::from_utf8;
use std::time::UNIX_EPOCH;

const PKG_NAME: &'static str = env!("CARGO_PKG_NAME");

fn get_env(s: &str) -> String{
    env::var(s).map_err(|e| format!("Error reading environment variable {}: {}", s, e)).unwrap()
}

#[derive(Debug, From)]
enum Error {
    Utf8Error(std::str::Utf8Error),
    JsonError(serde_json::Error),
    InvalidRuuvitagPacket(ruuvipacket::Error),
    TimeError(std::time::SystemTimeError),
    InfluxError(influx_db_client::Error),
}

fn decode_advert(p: &Publish) -> Result<BLEAdvert, Error> {
    let s = from_utf8(&p.payload)?;
    let advert: BLEAdvert = serde_json::from_str::<BLEAdvert>(s)?;
    Ok(advert)
}

fn influx_post(client: &mut influx_db_client::Client, pkt: &ruuvipacket::Packet, advert: &BLEAdvert) -> Result<(), Error> {
    use influx_db_client::{Point, Points, Value, Precision};
    let mut point = point!("ruuvi_measurements");
    point.add_timestamp(advert.time.duration_since(UNIX_EPOCH)?.as_secs() as i64);
    point.add_field("humidity", Value::Float(pkt.humidity));
    point.add_field("temperature", Value::Float(pkt.temperature));
    point.add_field("pressure", Value::Float(pkt.pressure));
    point.add_field("acceleration_x", Value::Float(pkt.acceleration_x));
    point.add_field("acceleration_y", Value::Float(pkt.acceleration_y));
    point.add_field("acceleration_z", Value::Float(pkt.acceleration_z));
    point.add_field("voltage", Value::Float(pkt.voltage));
    point.add_tag("mac", Value::String(advert.mac.clone()));
    point.add_tag("listener", Value::String(advert.listener.clone()));

    let points = points!(point);
    let _ = client.write_points(points, Some(Precision::Seconds), None)?;
    Ok(())
}

fn main() {
    let influx_host = get_env("INFLUX_HOST");
    let influx_db = get_env("INFLUX_DB");

    let mut influx_client = influx_db_client::Client::new(format!("http://{}:8086", influx_host), influx_db.to_string());

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
            let mut data = Cursor::new(&advert.manufacturer_data);
            if let Ok(packet) = ruuvipacket::decode(&mut data) {
                let result = influx_post(&mut influx_client, &packet, &advert);
                if result.is_err() {
                    println!("{:?}", result);
                }
            }
        }
    }
}
