use cookie_cutter::encoding::vanilla;
use dispatch_bundle::bundle;

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
    Set(State) = 0xca,
    Get = 0x11,

    NoOp = NOOP_ADDR,
}

#[bundle(Actor)]
#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum FromPeripheral {
    State = 0xaa,
    Fault = 0x1f,

    NoOp = NOOP_ADDR,
}

trait Actor {}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum State {
    On = 0x5e,
    Off = 0xed,
}

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum Fault {
    Temperature = 0xde,
    Current = 0xad,
}

#[derive(vanilla::SerializeIter)]
pub struct NoOp;

impl Actor for State {}
impl Actor for Fault {}
impl Actor for NoOp {}
