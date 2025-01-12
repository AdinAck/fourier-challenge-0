#![no_std]
#![no_main]

mod fmt;

use common::{
    command,
    types::{pump::PumpState, temperature::Temperature},
};
use cookie_cutter::SerializeIter;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embedded_command::command_buffer::CommandBuffer;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts, mode, peripherals,
    usart::{self, Uart},
};

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

static STATE: Mutex<ThreadModeRawMutex, (Temperature, PumpState)> =
    Mutex::new((25, PumpState::Off));

#[embassy_executor::task]
async fn temp(mut uart: Uart<'static, mode::Async>) {
    use command::temperature::{FromPeripheral, ToPeripheral};

    let mut cmd_buf = CommandBuffer::<256>::new();

    loop {
        let mut buf = [0; 1];
        let n = fmt::unwrap!(uart.read_until_idle(&mut buf).await);
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

        let temp = {
            let lock = STATE.lock().await;
            lock.0
        };

        fmt::unwrap!(
            FromPeripheral::Temperature(temp).serialize_iter(buf.iter_mut().inspect(|_| {
                n += 1;
            }))
        );
        fmt::debug!("{}", buf[..n]);
        fmt::unwrap!(uart.write(&buf[..n]).await);

        fmt::info!("sent temperature");
    }
}

#[embassy_executor::task]
async fn pump(mut uart: Uart<'static, mode::Async>) {
    use command::pump::{FromPeripheral, ToPeripheral};

    let mut cmd_buf = CommandBuffer::<256>::new();

    loop {
        let mut buf = [0; 1];
        let n = fmt::unwrap!(uart.read_until_idle(&mut buf).await);
        fmt::debug!("{}", buf[..n]);
        fmt::unwrap!(cmd_buf.ingest(buf[..n].iter()));

        let mut iter = cmd_buf.iter();

        let result = ToPeripheral::deserialize_iter(&mut iter);

        let cmd = match result {
            // special case
            Err(cookie_cutter::error::Error::EndOfInput) => continue,

            Ok(cmd) => cmd,
            Err(e) => fmt::panic!("{}", e),
        };

        let memento = iter.capture();
        cmd_buf.flush(memento);

        fmt::info!("received cmd: {}.", cmd);

        let mut buf = [0; 8];
        let mut n = 0;

        let outgoing = {
            let mut state = STATE.lock().await;

            match cmd {
                ToPeripheral::Get => FromPeripheral::PumpState(state.1),
                ToPeripheral::Set(new_state) => {
                    state.1 = new_state;

                    FromPeripheral::PumpState(state.1)
                }
            }
        };

        fmt::unwrap!(outgoing.serialize_iter(buf.iter_mut().inspect(|_| {
            n += 1;
        })));
        fmt::debug!("{}", buf[..n]);
        fmt::unwrap!(uart.write(&buf[..n]).await);

        fmt::info!("sent pump state");
    }
}

#[embassy_executor::task]
async fn simulator() {
    loop {
        {
            let mut state = STATE.lock().await;

            match state.1 {
                PumpState::Off => state.0 += 1,
                PumpState::On => state.0 -= 1,
            }
        }

        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut usart_cfg = usart::Config::default();
    usart_cfg.baudrate = 9600;

    let usart1 = fmt::unwrap!(Uart::new(
        p.USART1, p.PA10, p.PA9, Irqs, p.DMA1_CH1, p.DMA1_CH2, usart_cfg,
    ));

    let usart2 = fmt::unwrap!(Uart::new(
        p.USART2, p.PB4, p.PB3, Irqs, p.DMA1_CH3, p.DMA1_CH4, usart_cfg,
    ));

    spawner.must_spawn(temp(usart1));
    spawner.must_spawn(pump(usart2));
    spawner.must_spawn(simulator());
}
