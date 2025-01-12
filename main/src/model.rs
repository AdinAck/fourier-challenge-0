use heapless::HistoryBuffer;
use rtic_monotonics::Monotonic;

use common::types::{pump::PumpState, temperature::Temperature};

use crate::app::Mono;

pub struct Entry {
    #[allow(unused)] // unused in example implementation
    timestamp: <Mono as Monotonic>::Instant,
    temperature: Temperature,
    #[allow(unused)] // unused in example implementation
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

    pub fn try_push_pending(&mut self) {
        let Some(temperature) = self.pending.0 else {
            return;
        };
        let Some(pump_state) = self.pending.1 else {
            return;
        };

        self.history.write(Entry {
            timestamp: Mono::now(),
            temperature,
            pump_state,
        });

        self.pending = (None, None);
    }

    pub fn push_temperature(&mut self, temp: Temperature) {
        self.pending.0.replace(temp);

        self.try_push_pending();
    }

    pub fn push_pump_state(&mut self, state: PumpState) {
        self.pending.1.replace(state);

        self.try_push_pending();
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
