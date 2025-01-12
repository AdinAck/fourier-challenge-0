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
    use crate::{
        model::Model,
        peripherals::{pump::Pump, temperature::TempSensor},
    };

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

    pub type Tx2 = serial::usart::Tx<
        hal::pac::USART2,
        gpio::gpiob::PB3<gpio::Alternate<{ gpio::AF7 }>>,
        serial::NoDMA,
    >;

    pub type TransferIn1 = dma::Transfer<
        dma::stream::Stream0<hal::pac::DMA1>,
        serial::Rx<
            hal::pac::USART1,
            gpio::gpioa::PA10<gpio::Alternate<{ gpio::AF7 }>>,
            serial::DMA,
        >,
        dma::PeripheralToMemory,
        &'static mut [u8],
        dma::transfer::MutTransfer,
    >;

    pub type TransferIn2 = dma::Transfer<
        dma::stream::Stream1<hal::pac::DMA1>,
        serial::Rx<hal::pac::USART2, gpio::gpiob::PB4<gpio::Alternate<{ gpio::AF7 }>>, serial::DMA>,
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
        writer1: SignalWriter<'static, ()>,
        writer2: SignalWriter<'static, ()>,
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
        let gpiob = ctx.device.GPIOB.split(&mut rcc);

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

        let (tx2, rx2) = fmt::unwrap!(ctx.device.USART2.usart(
            gpiob.pb3.into_alternate(),
            gpiob.pb4.into_alternate(),
            usart_cfg,
            &mut rcc
        ))
        .split();

        let rx1_buf = {
            static mut BUF: [u8; 256] = [0; 256];

            // SAFETY: exclusive reference only
            #[allow(static_mut_refs)]
            unsafe {
                &mut BUF
            }
        };

        let rx2_buf = {
            static mut BUF: [u8; 256] = [0; 256];

            // SAFETY: exclusive reference only
            #[allow(static_mut_refs)]
            unsafe {
                &mut BUF
            }
        };

        let transfer_in_1 = streams.0.into_peripheral_to_memory_transfer(
            rx1.enable_dma(),
            &mut rx1_buf[..],
            dma_cfg,
        );

        let transfer_in_2 = streams.1.into_peripheral_to_memory_transfer(
            rx2.enable_dma(),
            &mut rx2_buf[..],
            dma_cfg,
        );

        let (writer1, reader1) = {
            static SIGNAL: Signal<()> = Signal::new();
            SIGNAL.split()
        };

        let (writer2, reader2) = {
            static SIGNAL: Signal<()> = Signal::new();
            SIGNAL.split()
        };

        if let Err(_) = temp::spawn(TempSensor::new(tx1, transfer_in_1, reader1)) {
            fmt::panic!("Failed to spawn task.")
        }

        if let Err(_) = pump::spawn(Pump::new(tx2, transfer_in_2, reader2)) {
            fmt::panic!("Failed to spawn task.")
        }

        (
            Shared {
                model: Model::new(60),
            },
            Local { writer1, writer2 },
        )
    }

    #[task(binds = USART1, local = [writer1])]
    fn usart1_event(ctx: usart1_event::Context) {
        ctx.local.writer1.write(());

        // terrible
        let usart1 = unsafe { &*hal::pac::USART1::ptr() };
        usart1.icr.write(|w| w.rtocf().set_bit());
    }

    #[task(binds = USART2, local = [writer2])]
    fn usart2_event(ctx: usart2_event::Context) {
        ctx.local.writer2.write(());

        // terrible
        let usart2 = unsafe { &*hal::pac::USART2::ptr() };
        usart2.icr.write(|w| w.rtocf().set_bit());
    }

    #[task(shared = [model])]
    async fn temp(ctx: temp::Context, mut temp_sensor: TempSensor) {
        // for testing purposes
        Mono::delay(4u64.secs()).await;

        fmt::info!("begin...");

        match temp_sensor.run(ctx.shared.model).await {
            Ok(_) => {
                // shutdown
            }
            Err(fault) => {
                // handle fault
                fmt::panic!("{}", fault);
            }
        }
    }

    #[task(shared = [model])]
    async fn pump(ctx: pump::Context, mut pump: Pump) {
        // for testing purposes
        Mono::delay(4u64.secs()).await;

        fmt::info!("begin...");

        match pump.run(ctx.shared.model).await {
            Ok(_) => {
                // shutdown
            }
            Err(fault) => {
                // handle fault
                fmt::panic!("{}", fault);
            }
        }
    }
}
