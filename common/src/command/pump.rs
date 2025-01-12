use cookie_cutter::encoding::vanilla;

use crate::types::pump::PumpState;

// NOTE: this pattern is used a lot,
// maybe i should make another higher level
// proc macro which combines bidirectional
// commands and separates them based on
// annotations, rather than relying on
// constants.

const NOOP_ADDR: u8 = 0xff;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum ToPeripheral {
    Set(PumpState) = 0xca,
    Get = 0x11,

    NoOp = NOOP_ADDR,
}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    PumpState(PumpState) = 0xaa,
    Fault(Fault) = 0x1f,

    NoOp = NOOP_ADDR,
}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum Fault {
    Temperature = 0xde,
    Current = 0xad,
}
