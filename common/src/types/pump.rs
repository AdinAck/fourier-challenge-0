use cookie_cutter::encoding::vanilla;

#[derive(Clone, Copy, PartialEq, vanilla::SerializeIter)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum PumpState {
    On = 0x5e,
    Off = 0xed,
}
