use cookie_cutter::encoding::vanilla;

#[derive(vanilla::SerializeIter)]
#[repr(u8)]
pub enum PumpState {
    On = 0x5e,
    Off = 0xed,
}
