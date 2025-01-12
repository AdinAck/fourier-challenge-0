#![no_std]
#![no_main]

mod fmt;

use common::command;
use cookie_cutter::SerializeIter;
use embedded_command::command_buffer::CommandBuffer;
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts, peripherals,
    usart::{self, Uart},
};

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    use command::temperature::{FromPeripheral, ToPeripheral};
    let p = embassy_stm32::init(Default::default());

    let mut usart_cfg = usart::Config::default();
    usart_cfg.baudrate = 9600;

    let mut usart1 = fmt::unwrap!(Uart::new(
        p.USART1, p.PA10, p.PA9, Irqs, p.DMA1_CH1, p.DMA1_CH2, usart_cfg,
    ));

    let mut cmd_buf = CommandBuffer::<256>::new();

    loop {
        let mut buf = [0; 1];
        let n = fmt::unwrap!(usart1.read_until_idle(&mut buf).await);
        fmt::debug!("{}", buf[..n]);
        fmt::unwrap!(cmd_buf.ingest(buf[..n].iter()));

        let mut iter = cmd_buf.iter();

        let result = ToPeripheral::deserialize_iter(&mut iter);

        let ToPeripheral::Read = match result {
            // special case
            Err(cookie_cutter::error::Error::EndOfInput) => continue,

            Ok(cmd) => cmd,
            Err(e) => fmt::panic!("{}", e),
        };

        let memento = iter.capture();
        cmd_buf.flush(memento);

        fmt::info!("received read.");

        let mut buf = [0; 8];
        let mut n = 0;

        fmt::unwrap!(
            FromPeripheral::Temperature(35).serialize_iter(buf.iter_mut().inspect(|_| {
                n += 1;
            }))
        );
        fmt::debug!("{}", buf[..n]);
        fmt::unwrap!(usart1.write(&buf[..n]).await);

        fmt::info!("sent temperature");
    }
}
