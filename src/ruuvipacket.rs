use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

#[derive(Debug, From)]
pub enum Error {
    IoError(std::io::Error),
    InvalidManufacturerId,
    InvalidVersion,
}

#[derive(Debug)]
pub struct Packet {
    pub version: u32,
    pub humidity: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub acceleration_x: f64,
    pub acceleration_y: f64,
    pub acceleration_z: f64,
    pub voltage: f64,
}

pub fn decode(buf: &mut std::io::Read) -> Result<Packet, Error> {
    if buf.read_u16::<LittleEndian>()? != 0x0499 {
        return Err(Error::InvalidManufacturerId);
    }
    let version = buf.read_u8()?;
    match version {
        3 => {
            let humidity = buf.read_u8()?;
            let temp_int = buf.read_u8()?;
            let temp_hundredths = buf.read_u8()?;
            let pressure = buf.read_u16::<BigEndian>()?;
        
            let accel_x = buf.read_i16::<BigEndian>()?;
            let accel_y = buf.read_i16::<BigEndian>()?;
            let accel_z = buf.read_i16::<BigEndian>()?;
            let battery_mv = buf.read_u16::<BigEndian>()?;
        
            let humidity = humidity as f64 / 2.0;
            let sign = match temp_int & 0x80 == 0 {
                true => 1.0,
                false => -1.0,
            };
            let temperature = sign * ((temp_int & !(0x80)) as f64 + temp_hundredths as f64 / 100.0);
            let pressure = pressure as f64 + 50000.0;
            let voltage = battery_mv as f64 / 1000.0;
        
            Ok(Packet{
                version: 3,
                humidity: humidity,
                temperature: temperature,
                pressure: pressure,
                acceleration_x: accel_x as f64,
                acceleration_y: accel_y as f64,
                acceleration_z: accel_z as f64,
                voltage: voltage,
            })
        },
        _ => Err(Error::InvalidVersion)
    }
}
