use cookie_cutter::encoding::vanilla;
use dispatch_bundle::bundle;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum ToPeripheral {
    On = 0xca,
    Off = 0x11,
}

#[bundle(Actor)]
#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    Fault,
}

trait Actor {}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum Fault {
    Temperature = 0xde,
    Current = 0xad,
}

impl Actor for Fault {}
