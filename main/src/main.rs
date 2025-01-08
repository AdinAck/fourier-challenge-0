#![no_std]
#![no_main]

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

    use hal::{
        prelude::*,
        pwr::{self, PwrExt as _},
        rcc::{self, RccExt as _},
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
