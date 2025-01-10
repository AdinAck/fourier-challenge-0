use cookie_cutter::encoding::vanilla;
use dispatch_bundle::bundle;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum ToPeripheral {
    Read = 0xbe,
}

#[bundle(Actor)]
#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    Temperature = 0xef,
}

trait Actor {}

#[derive(vanilla::SerializeIter)]
pub struct Temperature(i8);

impl Actor for Temperature {}
