use cookie_cutter::encoding::vanilla;
use dispatch_bundle::bundle;

use crate::peripherals::temperature::Measurement;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum ToPeripheral {
    /// Request a new measurement.
    Read = 0xbe,
}

#[bundle(Actor)]
#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    /// A temperature value in Celsius.
    Temperature = 0xef,
}

trait Actor {}

#[derive(vanilla::SerializeIter)]
pub struct Temperature(pub Measurement);

impl Actor for Temperature {}
