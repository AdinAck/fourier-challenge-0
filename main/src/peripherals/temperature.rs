use cookie_cutter::SerializeIter;
use embedded_command::command_buffer::CommandBuffer;
use futures::future::try_join;
use rtic::Mutex;
use rtic_monotonics::{fugit::ExtU64, Monotonic};
use rtic_sync::signal::SignalReader;

use crate::{
    app::{Mono, TransferIn1, Tx1},
    fmt,
    model::Model,
};
use common::{
    command::temperature::{FromPeripheral, ToPeripheral},
    types::temperature::Temperature,
};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    TransferInProgress,
    Ingestion(embedded_command::command_buffer::error::Overflow),
    Deserialize(cookie_cutter::error::Error),
    Timeout,
}

impl From<embedded_command::command_buffer::error::Overflow> for Error {
    fn from(value: embedded_command::command_buffer::error::Overflow) -> Self {
        Self::Ingestion(value)
    }
}

impl From<cookie_cutter::error::Error> for Error {
    fn from(value: cookie_cutter::error::Error) -> Self {
        Self::Deserialize(value)
    }
}

impl From<cookie_cutter::error::EndOfInput> for Error {
    fn from(_value: cookie_cutter::error::EndOfInput) -> Self {
        Self::Deserialize(cookie_cutter::error::Error::EndOfInput)
    }
}

impl From<rtic_monotonics::TimeoutError> for Error {
    fn from(_value: rtic_monotonics::TimeoutError) -> Self {
        Self::Timeout
    }
}

pub struct TempSensor {
    tx: Tx1,

    transfer_in: TransferIn1,

    signal: SignalReader<'static, ()>,
    command_buf: CommandBuffer<256>,
}

impl TempSensor {
    pub const fn new(tx: Tx1, transfer_in: TransferIn1, signal: SignalReader<'static, ()>) -> Self {
        Self {
            tx,
            transfer_in,

            signal,
            command_buf: CommandBuffer::new(),
        }
    }

    fn write_command(&mut self, command: ToPeripheral) -> Result<(), Error> {
        use stm32g4xx_hal::{block, hal::serial::Write as _};

        let mut buf = [0; 8];
        let mut n = 0;
        command.serialize_iter(buf.iter_mut().inspect(|_| {
            n += 1;
        }))?;

        for byte in &buf[..n] {
            fmt::unwrap!(block!(self.tx.write(*byte)));
        }

        fmt::unwrap!(block!(self.tx.flush()));

        Ok(())
    }

    async fn read_command(&mut self) -> Result<FromPeripheral, Error> {
        loop {
            self.signal.wait().await;

            self.transfer_in.peek_buffer(|buf, remaining| {
                fmt::trace!("buf: {}", buf[..buf.len() - remaining]);

                self.command_buf
                    .ingest(buf[..buf.len() - remaining].iter())?;

                Ok::<_, embedded_command::command_buffer::error::Overflow>(())
            })?;

            self.transfer_in.restart(|_| {});

            let mut iter = self.command_buf.iter();

            let result = FromPeripheral::deserialize_iter(&mut iter);

            let result = match result {
                // special case
                Err(cookie_cutter::error::Error::EndOfInput) => continue,

                Ok(cmd) => Ok(cmd),
                Err(e) => Err(e.into()),
            };

            let memento = iter.capture();
            self.command_buf.flush(memento);

            break result;
        }
    }

    pub async fn read_temperature(&mut self) -> Result<Temperature, Error> {
        // 1. send read command
        self.write_command(ToPeripheral::Read)?;
        fmt::trace!("sent read command");

        // 2. receive measurement command or timeout
        let FromPeripheral::Temperature(temp) =
            Mono::timeout_after(100u64.millis(), self.read_command()).await??;

        fmt::trace!("received temp: {}", temp);

        Ok(temp)
    }

    pub async fn run(&mut self, mut model: impl Mutex<T = Model>) -> Result<(), Error> {
        self.transfer_in.start(|_| {});

        loop {
            // 1. fetch latest measurement
            try_join(self.read_temperature(), async {
                Mono::delay(1u64.secs()).await;
                Ok(())
            })
            .await
            .and_then(|(measurement, _)| {
                // 2. update model
                model.lock(|model| {
                    model.push_temperature(measurement);
                });

                Ok(())
            })?;
        }
    }
}
