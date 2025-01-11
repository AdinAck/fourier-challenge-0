use cookie_cutter::SerializeIter;
use embedded_command::command_buffer::CommandBuffer;
use rtic_monotonics::{fugit::ExtU64, Monotonic};
use rtic_sync::signal::SignalReader;

use crate::{
    app::{Mono, TransferIn1, TransferOut1},
    command::temperature::{FromPeripheral, ToPeripheral},
    fmt,
};

pub type Measurement = i8;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    TransferInProgress,
    Ingestion(embedded_command::command_buffer::error::Overflow),
    Deserialize(cookie_cutter::error::Error),
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

pub struct Temperature {
    out_buf: *mut [u8],

    transfer_out: TransferOut1,
    transfer_in: TransferIn1,

    signal: SignalReader<'static, ()>,
    command_buf: CommandBuffer<256>,
}

impl Temperature {
    pub const fn new(
        transfer_out: TransferOut1,
        out_buf: *mut [u8],
        transfer_in: TransferIn1,
        signal: SignalReader<'static, ()>,
    ) -> Self {
        Self {
            out_buf,

            transfer_out,
            transfer_in,

            signal,
            command_buf: CommandBuffer::new(),
        }
    }

    fn write_command(&mut self, command: ToPeripheral) -> Result<(), Error> {
        let mut n = 0;
        fmt::unwrap!(
            command.serialize_iter(unsafe { &mut *self.out_buf }.iter_mut().inspect(|_| {
                n += 1;
            }))
        );
        self.transfer_out.restart(|_| {});

        Ok(())
    }

    async fn read_command(&mut self) -> Result<FromPeripheral, Error> {
        loop {
            self.transfer_in.peek_buffer(|buf, _remaining| {
                self.command_buf.ingest(buf.iter())?;

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

    pub async fn read_temperature(&mut self) -> Result<Measurement, Error> {
        // 1. send read command
        self.write_command(ToPeripheral::Read)?;

        // 2. receive measurement command
        let FromPeripheral::Temperature(crate::command::temperature::Temperature(temp)) =
            self.read_command().await?;

        Ok(temp)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.transfer_in.start(|_| {});

        loop {
            // 1. fetch latest measurement
            let measurement = self.read_temperature().await?;
            // 2. update model
            // 3. report faults

            Mono::delay(1u64.secs()).await;
        }
    }
}

unsafe impl Send for Temperature {}
