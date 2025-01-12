use heapless::HistoryBuffer;
use stm32g4xx_hal::time::Instant;

use common::types::{pump::PumpState, temperature::Temperature};

pub struct Entry {
    timestamp: Instant,
    temperature: Temperature,
    pump_state: PumpState,
}

pub struct Model {
    target_temp: Temperature,

    history: HistoryBuffer<Entry, 8>,
    pending: (Option<Temperature>, Option<PumpState>),
}

impl Model {
    pub const fn new(target_temp: Temperature) -> Self {
        Self {
            target_temp,
            history: HistoryBuffer::new(),
            pending: (None, None),
        }
    }

    pub fn push_temperature(&mut self, temp: Temperature) {
        self.pending.0.replace(temp);
    }

    pub fn pump_target(&self) -> PumpState {
        // some function of the history
        // will determine the appropriate
        // next pump state. knowing nothing
        // about the system i will use the
        // simplest possible control scheme

        let Some(entry) = self.history.last() else {
            // cool by default because likely
            // cool is safe
            return PumpState::On;
        };

        if entry.temperature > self.target_temp {
            PumpState::On
        } else {
            PumpState::Off
        }
    }
}
