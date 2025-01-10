#![no_std]
#![no_main]

mod command;
mod fmt;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::fmt;

    use stm32g4xx_hal as hal;

    // monotonics
    use rtic_monotonics::{fugit::ExtU64, stm32_tim2_monotonic, Monotonic};
    const MONO_FREQ: u32 = 31_250;
    stm32_tim2_monotonic!(Mono, MONO_FREQ);

    use hal::{prelude::*, pwr, rcc, serial, time};

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

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let pwr_cfg = ctx.device.PWR.constrain().vos(VOS_CFG).freeze();
        let mut rcc = ctx
            .device
            .RCC
            .constrain()
            .freeze(rcc::Config::pll().pll_cfg(PLL_CFG), pwr_cfg);

        fmt::debug!("{}", rcc.clocks);

        let gpioa = ctx.device.GPIOA.split(&mut rcc);

        let usart_cfg = serial::FullConfig::default().baudrate(time::Bps(9600)); // so strange

        // HAL: USART configuration validation should absolutely be const

        let usart1 = fmt::unwrap!(ctx.device.USART1.usart(
            gpioa.pa9.into_alternate(),
            gpioa.pa10.into_alternate(),
            usart_cfg,
            &mut rcc,
        ));

        let usart2 = fmt::unwrap!(ctx.device.USART2.usart(
            gpioa.pa2.into_alternate(),
            gpioa.pa3.into_alternate(),
            usart_cfg,
            &mut rcc
        ));

        // monotonic
        Mono::start(rcc.clocks.apb1_tim_clk.raw());
        #[cfg(feature = "defmt")]
        defmt::timestamp!("{}ms", Mono::now().ticks() * 1000 / MONO_FREQ as u64);

        fmt::unwrap!(hello::spawn());

        (Shared {}, Local {})
    }

    #[task]
    async fn hello(ctx: hello::Context) {
        loop {
            fmt::info!("Hello!");

            Mono::delay(1u64.secs()).await;
        }
    }
}
