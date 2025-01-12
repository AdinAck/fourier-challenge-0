use cookie_cutter::encoding::vanilla;

use crate::types::temperature::Temperature;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum ToPeripheral {
    /// Request a new measurement.
    Read = 0xbe,
}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    /// A temperature value in Celsius.
    Temperature(Temperature) = 0xef,
}
