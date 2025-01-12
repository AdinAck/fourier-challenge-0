#![no_std]
#![no_main]

mod fmt;
mod model;
mod peripherals;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use crate::{model::Model, peripherals::temperature::TempSensor};

    use super::fmt;

    // monotonics
    use rtic_monotonics::{fugit::ExtU64 as _, stm32_tim2_monotonic, Monotonic as _};
    const MONO_FREQ: u32 = 31_250;
    stm32_tim2_monotonic!(Mono, MONO_FREQ);

    use rtic_sync::signal::{Signal, SignalWriter};
    use stm32g4xx_hal::{
        self as hal,
        dma::{self, stream::DMAExt, TransferExt},
        gpio,
        prelude::*,
        pwr, rcc, serial, time,
    };

    // const configs
    const VOS_CFG: pwr::VoltageScale = pwr::VoltageScale::Range1 { enable_boost: true };

    const PLL_CFG: rcc::PllConfig = {
        use rcc::*;

        PllConfig {
            mux: PllSrc::HSI,        // 16MHz
            m: PllMDiv::DIV_4,       // /4 = 4MHz
            n: PllNMul::MUL_85,      // x85 = 340MHz
            r: Some(PllRDiv::DIV_2), // /2 = 170MHz
            q: None,
            p: None,
        }
    };

    // aliases
    pub type Tx1 = serial::usart::Tx<
        hal::pac::USART1,
        gpio::gpioa::PA9<gpio::Alternate<{ gpio::AF7 }>>,
        serial::NoDMA,
    >;

    pub type TransferIn1 = dma::Transfer<
        dma::stream::Stream1<hal::pac::DMA1>,
        serial::Rx<
            hal::pac::USART1,
            gpio::gpioa::PA10<gpio::Alternate<{ gpio::AF7 }>>,
            serial::DMA,
        >,
        dma::PeripheralToMemory,
        &'static mut [u8],
        dma::transfer::MutTransfer,
    >;

    #[shared]
    struct Shared {
        model: Model,
    }

    #[local]
    struct Local {
        writer: SignalWriter<'static, ()>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let pwr_cfg = ctx.device.PWR.constrain().vos(VOS_CFG).freeze();
        let mut rcc = ctx
            .device
            .RCC
            .constrain()
            .freeze(rcc::Config::pll().pll_cfg(PLL_CFG), pwr_cfg);

        // monotonic
        Mono::start(rcc.clocks.apb1_tim_clk.raw());
        #[cfg(feature = "defmt")]
        defmt::timestamp!("{}ms", Mono::now().ticks() * 1000 / MONO_FREQ as u64);

        fmt::debug!("{}", rcc.clocks);

        let streams = ctx.device.DMA1.split(&rcc);

        let dma_cfg = dma::config::DmaConfig::default()
            .transfer_complete_interrupt(true)
            .circular_buffer(true)
            .memory_increment(true);

        let gpioa = ctx.device.GPIOA.split(&mut rcc);

        let usart_cfg = serial::FullConfig::default()
            .baudrate(time::Bps(9600)) // so strange
            .receiver_timeout_us(1000);

        // HAL: USART configuration validation should absolutely be const

        let (tx1, rx1) = fmt::unwrap!(ctx.device.USART1.usart(
            gpioa.pa9.into_alternate(),
            gpioa.pa10.into_alternate(),
            usart_cfg,
            &mut rcc,
        ))
        .split();

        // let (tx2, rx2) = fmt::unwrap!(ctx.device.USART2.usart(
        //     gpioa.pa2.into_alternate(),
        //     gpioa.pa3.into_alternate(),
        //     usart_cfg,
        //     &mut rcc
        // ))
        // .split();

        let rx1_buf = {
            static mut BUF: [u8; 256] = [0; 256];

            // SAFETY: exclusive reference only
            #[allow(static_mut_refs)]
            unsafe {
                &mut BUF
            }
        };

        let transfer_in_1 = streams.1.into_peripheral_to_memory_transfer(
            rx1.enable_dma(),
            &mut rx1_buf[..],
            dma_cfg,
        );

        let (writer, reader) = {
            static SIGNAL: Signal<()> = Signal::new();
            SIGNAL.split()
        };

        if let Err(_) = temp::spawn(TempSensor::new(tx1, transfer_in_1, reader)) {
            fmt::panic!("Failed to spawn task.")
        }

        // fmt::unwrap!(pump::spawn());

        (
            Shared {
                model: Model::new(60),
            },
            Local { writer },
        )
    }

    #[task(binds = USART1, local = [writer])]
    fn usart1_event(ctx: usart1_event::Context) {
        ctx.local.writer.write(());

        // terrible
        let usart1 = unsafe { &*hal::pac::USART1::ptr() };
        usart1.icr.write(|w| w.rtocf().set_bit());
    }

    #[task(shared = [model])]
    async fn temp(ctx: temp::Context, mut temp_sensor: TempSensor) {
        Mono::delay(4u64.secs()).await;

        fmt::info!("begin...");

        match temp_sensor.run(ctx.shared.model).await {
            Ok(_) => {
                // shutdown
            }
            Err(fault) => {
                // handle fault
                fmt::error!("{}", fault);
            }
        }
    }

    #[task]
    async fn pump(_ctx: pump::Context) {
        loop {
            // 1. ask model for target pump state
            // 2. send pump state to pump
            // 3. validate pump response
            // 4. report faults

            fmt::info!("hello!");

            Mono::delay(200u64.millis()).await;
        }
    }
}
