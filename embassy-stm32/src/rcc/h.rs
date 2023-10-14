use core::ops::RangeInclusive;

use crate::pac;
use crate::pac::pwr::vals::Vos;
#[cfg(stm32h5)]
pub use crate::pac::rcc::vals::Adcdacsel as AdcClockSource;
#[cfg(stm32h7)]
pub use crate::pac::rcc::vals::Adcsel as AdcClockSource;
use crate::pac::rcc::vals::{Ckpersel, Hsidiv, Pllrge, Pllsrc, Pllvcosel, Sw, Timpre};
pub use crate::pac::rcc::vals::{Ckpersel as PerClockSource, Plldiv as PllDiv, Pllm as PllPreDiv, Plln as PllMul};
use crate::pac::{FLASH, PWR, RCC};
use crate::rcc::{set_freqs, Clocks};
use crate::time::Hertz;

/// HSI speed
pub const HSI_FREQ: Hertz = Hertz(64_000_000);

/// CSI speed
pub const CSI_FREQ: Hertz = Hertz(4_000_000);

/// HSI48 speed
pub const HSI48_FREQ: Hertz = Hertz(48_000_000);

const VCO_RANGE: RangeInclusive<Hertz> = Hertz(150_000_000)..=Hertz(420_000_000);
#[cfg(any(stm32h5, pwr_h7rm0455))]
const VCO_WIDE_RANGE: RangeInclusive<Hertz> = Hertz(128_000_000)..=Hertz(560_000_000);
#[cfg(pwr_h7rm0468)]
const VCO_WIDE_RANGE: RangeInclusive<Hertz> = Hertz(192_000_000)..=Hertz(836_000_000);
#[cfg(any(pwr_h7rm0399, pwr_h7rm0433))]
const VCO_WIDE_RANGE: RangeInclusive<Hertz> = Hertz(192_000_000)..=Hertz(960_000_000);

pub use crate::pac::rcc::vals::{Hpre as AHBPrescaler, Ppre as APBPrescaler};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VoltageScale {
    Scale0,
    Scale1,
    Scale2,
    Scale3,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum HseMode {
    /// crystal/ceramic oscillator (HSEBYP=0)
    Oscillator,
    /// external analog clock (low swing) (HSEBYP=1, HSEEXT=0)
    Bypass,
    /// external digital clock (full swing) (HSEBYP=1, HSEEXT=1)
    #[cfg(any(rcc_h5, rcc_h50))]
    BypassDigital,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Hse {
    /// HSE frequency.
    pub freq: Hertz,
    /// HSE mode.
    pub mode: HseMode,
}

#[cfg(stm32h7)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Lse {
    /// 32.768 kHz crystal/ceramic oscillator (LSEBYP=0)
    Oscillator,
    /// external clock input up to 1MHz (LSEBYP=1)
    Bypass(Hertz),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Hsi {
    /// 64Mhz
    Mhz64,
    /// 32Mhz (divided by 2)
    Mhz32,
    /// 16Mhz (divided by 4)
    Mhz16,
    /// 8Mhz (divided by 8)
    Mhz8,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Sysclk {
    /// HSI selected as sysclk
    HSI,
    /// HSE selected as sysclk
    HSE,
    /// CSI selected as sysclk
    CSI,
    /// PLL1_P selected as sysclk
    Pll1P,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PllSource {
    Hsi,
    Csi,
    Hse,
}

#[derive(Clone, Copy)]
pub struct Pll {
    /// Source clock selection.
    #[cfg(stm32h5)]
    pub source: PllSource,

    /// PLL pre-divider (DIVM).
    pub prediv: PllPreDiv,

    /// PLL multiplication factor.
    pub mul: PllMul,

    /// PLL P division factor. If None, PLL P output is disabled.
    /// On PLL1, it must be even (in particular, it cannot be 1.)
    pub divp: Option<PllDiv>,
    /// PLL Q division factor. If None, PLL Q output is disabled.
    pub divq: Option<PllDiv>,
    /// PLL R division factor. If None, PLL R output is disabled.
    pub divr: Option<PllDiv>,
}

fn apb_div_tim(apb: &APBPrescaler, clk: Hertz, tim: TimerPrescaler) -> Hertz {
    match (tim, apb) {
        (TimerPrescaler::DefaultX2, APBPrescaler::DIV1) => clk,
        (TimerPrescaler::DefaultX2, APBPrescaler::DIV2) => clk,
        (TimerPrescaler::DefaultX2, APBPrescaler::DIV4) => clk / 2u32,
        (TimerPrescaler::DefaultX2, APBPrescaler::DIV8) => clk / 4u32,
        (TimerPrescaler::DefaultX2, APBPrescaler::DIV16) => clk / 8u32,

        (TimerPrescaler::DefaultX4, APBPrescaler::DIV1) => clk,
        (TimerPrescaler::DefaultX4, APBPrescaler::DIV2) => clk,
        (TimerPrescaler::DefaultX4, APBPrescaler::DIV4) => clk,
        (TimerPrescaler::DefaultX4, APBPrescaler::DIV8) => clk / 2u32,
        (TimerPrescaler::DefaultX4, APBPrescaler::DIV16) => clk / 4u32,

        _ => unreachable!(),
    }
}

/// Timer prescaler
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TimerPrescaler {
    /// The timers kernel clock is equal to hclk if PPREx corresponds to a
    /// division by 1 or 2, else it is equal to 2*pclk
    DefaultX2,

    /// The timers kernel clock is equal to hclk if PPREx corresponds to a
    /// division by 1, 2 or 4, else it is equal to 4*pclk
    DefaultX4,
}

impl From<TimerPrescaler> for Timpre {
    fn from(value: TimerPrescaler) -> Self {
        match value {
            TimerPrescaler::DefaultX2 => Timpre::DEFAULTX2,
            TimerPrescaler::DefaultX4 => Timpre::DEFAULTX4,
        }
    }
}

/// Configuration of the core clocks
#[non_exhaustive]
pub struct Config {
    pub hsi: Option<Hsi>,
    pub hse: Option<Hse>,
    pub csi: bool,
    pub hsi48: bool,
    pub sys: Sysclk,

    #[cfg(stm32h7)]
    pub pll_src: PllSource,

    pub pll1: Option<Pll>,
    pub pll2: Option<Pll>,
    #[cfg(any(rcc_h5, stm32h7))]
    pub pll3: Option<Pll>,

    pub d1c_pre: AHBPrescaler,
    pub ahb_pre: AHBPrescaler,
    pub apb1_pre: APBPrescaler,
    pub apb2_pre: APBPrescaler,
    pub apb3_pre: APBPrescaler,
    #[cfg(stm32h7)]
    pub apb4_pre: APBPrescaler,

    pub per_clock_source: PerClockSource,
    pub adc_clock_source: AdcClockSource,
    pub timer_prescaler: TimerPrescaler,
    pub voltage_scale: VoltageScale,
    pub ls: super::LsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hsi: Some(Hsi::Mhz64),
            hse: None,
            csi: false,
            hsi48: false,
            sys: Sysclk::HSI,
            #[cfg(stm32h7)]
            pll_src: PllSource::Hsi,
            pll1: None,
            pll2: None,
            #[cfg(any(rcc_h5, stm32h7))]
            pll3: None,

            d1c_pre: AHBPrescaler::DIV1,
            ahb_pre: AHBPrescaler::DIV1,
            apb1_pre: APBPrescaler::DIV1,
            apb2_pre: APBPrescaler::DIV1,
            apb3_pre: APBPrescaler::DIV1,
            #[cfg(stm32h7)]
            apb4_pre: APBPrescaler::DIV1,

            per_clock_source: PerClockSource::HSI,
            adc_clock_source: AdcClockSource::from_bits(0), // PLL2_P on H7, HCLK on H5
            timer_prescaler: TimerPrescaler::DefaultX2,
            voltage_scale: VoltageScale::Scale0,
            ls: Default::default(),
        }
    }
}

pub(crate) unsafe fn init(config: Config) {
    // NB. The lower bytes of CR3 can only be written once after
    // POR, and must be written with a valid combination. Refer to
    // RM0433 Rev 7 6.8.4. This is partially enforced by dropping
    // `self` at the end of this method, but of course we cannot
    // know what happened between the previous POR and here.
    #[cfg(pwr_h7rm0433)]
    PWR.cr3().modify(|w| {
        w.set_scuen(true);
        w.set_ldoen(true);
        w.set_bypass(false);
    });

    #[cfg(any(pwr_h7rm0399, pwr_h7rm0455, pwr_h7rm0468))]
    PWR.cr3().modify(|w| {
        // hardcode "Direct SPMS" for now, this is what works on nucleos with the
        // default solderbridge configuration.
        w.set_sden(true);
        w.set_ldoen(false);
    });

    // Validate the supply configuration. If you are stuck here, it is
    // because the voltages on your board do not match those specified
    // in the D3CR.VOS and CR3.SDLEVEL fields. By default after reset
    // VOS = Scale 3, so check that the voltage on the VCAP pins =
    // 1.0V.
    #[cfg(any(pwr_h7rm0433, pwr_h7rm0399, pwr_h7rm0455, pwr_h7rm0468))]
    while !PWR.csr1().read().actvosrdy() {}

    // Configure voltage scale.
    #[cfg(any(pwr_h5, pwr_h50))]
    {
        PWR.voscr().modify(|w| {
            w.set_vos(match config.voltage_scale {
                VoltageScale::Scale0 => Vos::SCALE0,
                VoltageScale::Scale1 => Vos::SCALE1,
                VoltageScale::Scale2 => Vos::SCALE2,
                VoltageScale::Scale3 => Vos::SCALE3,
            })
        });
        while !PWR.vossr().read().vosrdy() {}
    }

    #[cfg(syscfg_h7)]
    {
        // in chips without the overdrive bit, we can go from any scale to any scale directly.
        PWR.d3cr().modify(|w| {
            w.set_vos(match config.voltage_scale {
                VoltageScale::Scale0 => Vos::SCALE0,
                VoltageScale::Scale1 => Vos::SCALE1,
                VoltageScale::Scale2 => Vos::SCALE2,
                VoltageScale::Scale3 => Vos::SCALE3,
            })
        });
        while !PWR.d3cr().read().vosrdy() {}
    }

    #[cfg(syscfg_h7od)]
    {
        match config.voltage_scale {
            VoltageScale::Scale0 => {
                // to go to scale0, we must go to Scale1 first...
                PWR.d3cr().modify(|w| w.set_vos(Vos::SCALE1));
                while !PWR.d3cr().read().vosrdy() {}

                // Then enable overdrive.
                critical_section::with(|_| pac::SYSCFG.pwrcr().modify(|w| w.set_oden(1)));
                while !PWR.d3cr().read().vosrdy() {}
            }
            _ => {
                // for all other scales, we can go directly.
                PWR.d3cr().modify(|w| {
                    w.set_vos(match config.voltage_scale {
                        VoltageScale::Scale0 => unreachable!(),
                        VoltageScale::Scale1 => Vos::SCALE1,
                        VoltageScale::Scale2 => Vos::SCALE2,
                        VoltageScale::Scale3 => Vos::SCALE3,
                    })
                });
                while !PWR.d3cr().read().vosrdy() {}
            }
        }
    }

    // Configure HSI
    let hsi = match config.hsi {
        None => {
            RCC.cr().modify(|w| w.set_hsion(false));
            None
        }
        Some(hsi) => {
            let (freq, hsidiv) = match hsi {
                Hsi::Mhz64 => (HSI_FREQ / 1u32, Hsidiv::DIV1),
                Hsi::Mhz32 => (HSI_FREQ / 2u32, Hsidiv::DIV2),
                Hsi::Mhz16 => (HSI_FREQ / 4u32, Hsidiv::DIV4),
                Hsi::Mhz8 => (HSI_FREQ / 8u32, Hsidiv::DIV8),
            };
            RCC.cr().modify(|w| {
                w.set_hsidiv(hsidiv);
                w.set_hsion(true);
            });
            while !RCC.cr().read().hsirdy() {}
            Some(freq)
        }
    };

    // Configure HSE
    let hse = match config.hse {
        None => {
            RCC.cr().modify(|w| w.set_hseon(false));
            None
        }
        Some(hse) => {
            RCC.cr().modify(|w| {
                w.set_hsebyp(hse.mode != HseMode::Oscillator);
                #[cfg(any(rcc_h5, rcc_h50))]
                w.set_hseext(match hse.mode {
                    HseMode::Oscillator | HseMode::Bypass => pac::rcc::vals::Hseext::ANALOG,
                    HseMode::BypassDigital => pac::rcc::vals::Hseext::DIGITAL,
                });
            });
            RCC.cr().modify(|w| w.set_hseon(true));
            while !RCC.cr().read().hserdy() {}
            Some(hse.freq)
        }
    };

    // Configure HSI48.
    RCC.cr().modify(|w| w.set_hsi48on(config.hsi48));
    let _hsi48 = match config.hsi48 {
        false => None,
        true => {
            while !RCC.cr().read().hsi48rdy() {}
            Some(CSI_FREQ)
        }
    };

    // Configure CSI.
    RCC.cr().modify(|w| w.set_csion(config.csi));
    let csi = match config.csi {
        false => None,
        true => {
            while !RCC.cr().read().csirdy() {}
            Some(CSI_FREQ)
        }
    };

    // Configure PLLs.
    let pll_input = PllInput {
        csi,
        hse,
        hsi,
        #[cfg(stm32h7)]
        source: config.pll_src,
    };
    let pll1 = init_pll(0, config.pll1, &pll_input);
    let pll2 = init_pll(1, config.pll2, &pll_input);
    #[cfg(any(rcc_h5, stm32h7))]
    let pll3 = init_pll(2, config.pll3, &pll_input);

    // Configure sysclk
    let (sys, sw) = match config.sys {
        Sysclk::HSI => (unwrap!(hsi), Sw::HSI),
        Sysclk::HSE => (unwrap!(hse), Sw::HSE),
        Sysclk::CSI => (unwrap!(csi), Sw::CSI),
        Sysclk::Pll1P => (unwrap!(pll1.p), Sw::PLL1),
    };

    // Check limits.
    #[cfg(stm32h5)]
    let (hclk_max, pclk_max) = match config.voltage_scale {
        VoltageScale::Scale0 => (Hertz(250_000_000), Hertz(250_000_000)),
        VoltageScale::Scale1 => (Hertz(200_000_000), Hertz(200_000_000)),
        VoltageScale::Scale2 => (Hertz(150_000_000), Hertz(150_000_000)),
        VoltageScale::Scale3 => (Hertz(100_000_000), Hertz(100_000_000)),
    };
    #[cfg(stm32h7)]
    let (d1cpre_clk_max, hclk_max, pclk_max) = match config.voltage_scale {
        VoltageScale::Scale0 => (Hertz(480_000_000), Hertz(240_000_000), Hertz(120_000_000)),
        VoltageScale::Scale1 => (Hertz(400_000_000), Hertz(200_000_000), Hertz(100_000_000)),
        VoltageScale::Scale2 => (Hertz(300_000_000), Hertz(150_000_000), Hertz(75_000_000)),
        VoltageScale::Scale3 => (Hertz(200_000_000), Hertz(100_000_000), Hertz(50_000_000)),
    };

    #[cfg(stm32h7)]
    let hclk = {
        let d1cpre_clk = sys / config.d1c_pre;
        assert!(d1cpre_clk <= d1cpre_clk_max);
        sys / config.ahb_pre
    };
    #[cfg(stm32h5)]
    let hclk = sys / config.ahb_pre;
    assert!(hclk <= hclk_max);

    let apb1 = hclk / config.apb1_pre;
    let apb1_tim = apb_div_tim(&config.apb1_pre, hclk, config.timer_prescaler);
    assert!(apb1 <= pclk_max);
    let apb2 = hclk / config.apb2_pre;
    let apb2_tim = apb_div_tim(&config.apb2_pre, hclk, config.timer_prescaler);
    assert!(apb2 <= pclk_max);
    let apb3 = hclk / config.apb3_pre;
    assert!(apb3 <= pclk_max);
    #[cfg(stm32h7)]
    let apb4 = hclk / config.apb4_pre;
    #[cfg(stm32h7)]
    assert!(apb4 <= pclk_max);

    let _per_ck = match config.per_clock_source {
        Ckpersel::HSI => hsi,
        Ckpersel::CSI => csi,
        Ckpersel::HSE => hse,
        _ => unreachable!(),
    };

    #[cfg(stm32h7)]
    let adc = match config.adc_clock_source {
        AdcClockSource::PLL2_P => pll2.p,
        AdcClockSource::PLL3_R => pll3.r,
        AdcClockSource::PER => _per_ck,
        _ => unreachable!(),
    };
    #[cfg(stm32h5)]
    let adc = match config.adc_clock_source {
        AdcClockSource::HCLK => Some(hclk),
        AdcClockSource::SYSCLK => Some(sys),
        AdcClockSource::PLL2_R => pll2.r,
        AdcClockSource::HSE => hse,
        AdcClockSource::HSI => hsi,
        AdcClockSource::CSI => csi,
        _ => unreachable!(),
    };

    flash_setup(hclk, config.voltage_scale);

    let rtc = config.ls.init();

    #[cfg(stm32h7)]
    {
        RCC.d1cfgr().modify(|w| {
            w.set_d1cpre(config.d1c_pre);
            w.set_d1ppre(config.apb3_pre);
            w.set_hpre(config.ahb_pre);
        });
        // Ensure core prescaler value is valid before future lower core voltage
        while RCC.d1cfgr().read().d1cpre() != config.d1c_pre {}

        RCC.d2cfgr().modify(|w| {
            w.set_d2ppre1(config.apb1_pre);
            w.set_d2ppre2(config.apb2_pre);
        });
        RCC.d3cfgr().modify(|w| {
            w.set_d3ppre(config.apb4_pre);
        });

        RCC.d1ccipr().modify(|w| {
            w.set_ckpersel(config.per_clock_source);
        });
        RCC.d3ccipr().modify(|w| {
            w.set_adcsel(config.adc_clock_source);
        });
    }
    #[cfg(stm32h5)]
    {
        // Set hpre
        RCC.cfgr2().modify(|w| w.set_hpre(config.ahb_pre));
        while RCC.cfgr2().read().hpre() != config.ahb_pre {}

        // set ppre
        RCC.cfgr2().modify(|w| {
            w.set_ppre1(config.apb1_pre);
            w.set_ppre2(config.apb2_pre);
            w.set_ppre3(config.apb3_pre);
        });

        RCC.ccipr5().modify(|w| {
            w.set_ckpersel(config.per_clock_source);
            w.set_adcdacsel(config.adc_clock_source)
        });
    }

    RCC.cfgr().modify(|w| w.set_timpre(config.timer_prescaler.into()));

    RCC.cfgr().modify(|w| w.set_sw(sw));
    while RCC.cfgr().read().sws() != sw {}

    // IO compensation cell - Requires CSI clock and SYSCFG
    #[cfg(stm32h7)] // TODO h5
    if csi.is_some() {
        // Enable the compensation cell, using back-bias voltage code
        // provide by the cell.
        critical_section::with(|_| {
            pac::SYSCFG.cccsr().modify(|w| {
                w.set_en(true);
                w.set_cs(false);
                w.set_hslv(false);
            })
        });
        while !pac::SYSCFG.cccsr().read().ready() {}
    }

    set_freqs(Clocks {
        sys,
        ahb1: hclk,
        ahb2: hclk,
        ahb3: hclk,
        ahb4: hclk,
        apb1,
        apb2,
        apb3,
        #[cfg(stm32h7)]
        apb4,
        #[cfg(stm32h5)]
        apb4: Hertz(1),
        apb1_tim,
        apb2_tim,
        adc,
        rtc,

        #[cfg(stm32h5)]
        hsi: None,
        #[cfg(stm32h5)]
        hsi48: None,
        #[cfg(stm32h5)]
        lsi: None,
        #[cfg(stm32h5)]
        csi: None,

        #[cfg(stm32h5)]
        lse: None,
        #[cfg(stm32h5)]
        hse: None,

        #[cfg(stm32h5)]
        pll1_q: pll1.q,
        #[cfg(stm32h5)]
        pll2_q: pll2.q,
        #[cfg(stm32h5)]
        pll2_p: pll2.p,
        #[cfg(stm32h5)]
        pll2_r: pll2.r,
        #[cfg(rcc_h5)]
        pll3_p: pll3.p,
        #[cfg(rcc_h5)]
        pll3_q: pll3.q,
        #[cfg(rcc_h5)]
        pll3_r: pll3.r,
        #[cfg(stm32h5)]
        pll3_1: None,

        #[cfg(rcc_h50)]
        pll3_p: None,
        #[cfg(rcc_h50)]
        pll3_q: None,
        #[cfg(rcc_h50)]
        pll3_r: None,

        #[cfg(stm32h5)]
        audioclk: None,
        #[cfg(stm32h5)]
        per: None,
    });
}

struct PllInput {
    hsi: Option<Hertz>,
    hse: Option<Hertz>,
    csi: Option<Hertz>,
    #[cfg(stm32h7)]
    source: PllSource,
}

struct PllOutput {
    p: Option<Hertz>,
    #[allow(dead_code)]
    q: Option<Hertz>,
    #[allow(dead_code)]
    r: Option<Hertz>,
}

fn init_pll(num: usize, config: Option<Pll>, input: &PllInput) -> PllOutput {
    let Some(config) = config else {
        // Stop PLL
        RCC.cr().modify(|w| w.set_pllon(num, false));
        while RCC.cr().read().pllrdy(num) {}

        // "To save power when PLL1 is not used, the value of PLL1M must be set to 0.""
        #[cfg(stm32h7)]
        RCC.pllckselr().write(|w| w.set_divm(num, PllPreDiv::from_bits(0)));
        #[cfg(stm32h5)]
        RCC.pllcfgr(num).write(|w| w.set_divm(PllPreDiv::from_bits(0)));

        return PllOutput {
            p: None,
            q: None,
            r: None,
        };
    };

    #[cfg(stm32h5)]
    let source = config.source;
    #[cfg(stm32h7)]
    let source = input.source;

    let (in_clk, src) = match source {
        PllSource::Hsi => (unwrap!(input.hsi), Pllsrc::HSI),
        PllSource::Hse => (unwrap!(input.hse), Pllsrc::HSE),
        PllSource::Csi => (unwrap!(input.csi), Pllsrc::CSI),
    };

    let ref_clk = in_clk / config.prediv as u32;

    let ref_range = match ref_clk.0 {
        ..=1_999_999 => Pllrge::RANGE1,
        ..=3_999_999 => Pllrge::RANGE2,
        ..=7_999_999 => Pllrge::RANGE4,
        ..=16_000_000 => Pllrge::RANGE8,
        x => panic!("pll ref_clk out of range: {} mhz", x),
    };

    // The smaller range (150 to 420 MHz) must
    // be chosen when the reference clock frequency is lower than 2 MHz.
    let wide_allowed = ref_range != Pllrge::RANGE1;

    let vco_clk = ref_clk * config.mul;
    let vco_range = if VCO_RANGE.contains(&vco_clk) {
        Pllvcosel::MEDIUMVCO
    } else if wide_allowed && VCO_WIDE_RANGE.contains(&vco_clk) {
        Pllvcosel::WIDEVCO
    } else {
        panic!("pll vco_clk out of range: {} mhz", vco_clk.0)
    };

    let p = config.divp.map(|div| {
        if num == 0 {
            // on PLL1, DIVP must be even.
            // The enum value is 1 less than the divider, so check it's odd.
            assert!(div.to_bits() % 2 == 1);
        }

        vco_clk / div
    });
    let q = config.divq.map(|div| vco_clk / div);
    let r = config.divr.map(|div| vco_clk / div);

    #[cfg(stm32h5)]
    RCC.pllcfgr(num).write(|w| {
        w.set_pllsrc(src);
        w.set_divm(config.prediv);
        w.set_pllvcosel(vco_range);
        w.set_pllrge(ref_range);
        w.set_pllfracen(false);
        w.set_pllpen(p.is_some());
        w.set_pllqen(q.is_some());
        w.set_pllren(r.is_some());
    });

    #[cfg(stm32h7)]
    {
        RCC.pllckselr().modify(|w| {
            w.set_divm(num, config.prediv);
            w.set_pllsrc(src);
        });
        RCC.pllcfgr().modify(|w| {
            w.set_pllvcosel(num, vco_range);
            w.set_pllrge(num, ref_range);
            w.set_pllfracen(num, false);
            w.set_divpen(num, p.is_some());
            w.set_divqen(num, q.is_some());
            w.set_divren(num, r.is_some());
        });
    }

    RCC.plldivr(num).write(|w| {
        w.set_plln(config.mul);
        w.set_pllp(config.divp.unwrap_or(PllDiv::DIV2));
        w.set_pllq(config.divq.unwrap_or(PllDiv::DIV2));
        w.set_pllr(config.divr.unwrap_or(PllDiv::DIV2));
    });

    RCC.cr().modify(|w| w.set_pllon(num, true));
    while !RCC.cr().read().pllrdy(num) {}

    PllOutput { p, q, r }
}

fn flash_setup(clk: Hertz, vos: VoltageScale) {
    // RM0481 Rev 1, table 37
    // LATENCY  WRHIGHFREQ  VOS3           VOS2            VOS1            VOS0
    //      0           0   0 to 20 MHz    0 to 30 MHz     0 to 34 MHz     0 to 42 MHz
    //      1           0   20 to 40 MHz   30 to 60 MHz    34 to 68 MHz    42 to 84 MHz
    //      2           1   40 to 60 MHz   60 to 90 MHz    68 to 102 MHz   84 to 126 MHz
    //      3           1   60 to 80 MHz   90 to 120 MHz   102 to 136 MHz  126 to 168 MHz
    //      4           2   80 to 100 MHz  120 to 150 MHz  136 to 170 MHz  168 to 210 MHz
    //      5           2                                  170 to 200 MHz  210 to 250 MHz
    #[cfg(stm32h5)]
    let (latency, wrhighfreq) = match (vos, clk.0) {
        (VoltageScale::Scale0, ..=42_000_000) => (0, 0),
        (VoltageScale::Scale0, ..=84_000_000) => (1, 0),
        (VoltageScale::Scale0, ..=126_000_000) => (2, 1),
        (VoltageScale::Scale0, ..=168_000_000) => (3, 1),
        (VoltageScale::Scale0, ..=210_000_000) => (4, 2),
        (VoltageScale::Scale0, ..=250_000_000) => (5, 2),

        (VoltageScale::Scale1, ..=34_000_000) => (0, 0),
        (VoltageScale::Scale1, ..=68_000_000) => (1, 0),
        (VoltageScale::Scale1, ..=102_000_000) => (2, 1),
        (VoltageScale::Scale1, ..=136_000_000) => (3, 1),
        (VoltageScale::Scale1, ..=170_000_000) => (4, 2),
        (VoltageScale::Scale1, ..=200_000_000) => (5, 2),

        (VoltageScale::Scale2, ..=30_000_000) => (0, 0),
        (VoltageScale::Scale2, ..=60_000_000) => (1, 0),
        (VoltageScale::Scale2, ..=90_000_000) => (2, 1),
        (VoltageScale::Scale2, ..=120_000_000) => (3, 1),
        (VoltageScale::Scale2, ..=150_000_000) => (4, 2),

        (VoltageScale::Scale3, ..=20_000_000) => (0, 0),
        (VoltageScale::Scale3, ..=40_000_000) => (1, 0),
        (VoltageScale::Scale3, ..=60_000_000) => (2, 1),
        (VoltageScale::Scale3, ..=80_000_000) => (3, 1),
        (VoltageScale::Scale3, ..=100_000_000) => (4, 2),

        _ => unreachable!(),
    };

    #[cfg(flash_h7)]
    let (latency, wrhighfreq) = match (vos, clk.0) {
        // VOS 0 range VCORE 1.26V - 1.40V
        (VoltageScale::Scale0, ..=70_000_000) => (0, 0),
        (VoltageScale::Scale0, ..=140_000_000) => (1, 1),
        (VoltageScale::Scale0, ..=185_000_000) => (2, 1),
        (VoltageScale::Scale0, ..=210_000_000) => (2, 2),
        (VoltageScale::Scale0, ..=225_000_000) => (3, 2),
        (VoltageScale::Scale0, ..=240_000_000) => (4, 2),
        // VOS 1 range VCORE 1.15V - 1.26V
        (VoltageScale::Scale1, ..=70_000_000) => (0, 0),
        (VoltageScale::Scale1, ..=140_000_000) => (1, 1),
        (VoltageScale::Scale1, ..=185_000_000) => (2, 1),
        (VoltageScale::Scale1, ..=210_000_000) => (2, 2),
        (VoltageScale::Scale1, ..=225_000_000) => (3, 2),
        // VOS 2 range VCORE 1.05V - 1.15V
        (VoltageScale::Scale2, ..=55_000_000) => (0, 0),
        (VoltageScale::Scale2, ..=110_000_000) => (1, 1),
        (VoltageScale::Scale2, ..=165_000_000) => (2, 1),
        (VoltageScale::Scale2, ..=224_000_000) => (3, 2),
        // VOS 3 range VCORE 0.95V - 1.05V
        (VoltageScale::Scale3, ..=45_000_000) => (0, 0),
        (VoltageScale::Scale3, ..=90_000_000) => (1, 1),
        (VoltageScale::Scale3, ..=135_000_000) => (2, 1),
        (VoltageScale::Scale3, ..=180_000_000) => (3, 2),
        (VoltageScale::Scale3, ..=224_000_000) => (4, 2),
        _ => unreachable!(),
    };

    // See RM0455 Rev 10 Table 16. FLASH recommended number of wait
    // states and programming delay
    #[cfg(flash_h7ab)]
    let (latency, wrhighfreq) = match (vos, clk.0) {
        // VOS 0 range VCORE 1.25V - 1.35V
        (VoltageScale::Scale0, ..=42_000_000) => (0, 0),
        (VoltageScale::Scale0, ..=84_000_000) => (1, 0),
        (VoltageScale::Scale0, ..=126_000_000) => (2, 1),
        (VoltageScale::Scale0, ..=168_000_000) => (3, 1),
        (VoltageScale::Scale0, ..=210_000_000) => (4, 2),
        (VoltageScale::Scale0, ..=252_000_000) => (5, 2),
        (VoltageScale::Scale0, ..=280_000_000) => (6, 3),
        // VOS 1 range VCORE 1.15V - 1.25V
        (VoltageScale::Scale1, ..=38_000_000) => (0, 0),
        (VoltageScale::Scale1, ..=76_000_000) => (1, 0),
        (VoltageScale::Scale1, ..=114_000_000) => (2, 1),
        (VoltageScale::Scale1, ..=152_000_000) => (3, 1),
        (VoltageScale::Scale1, ..=190_000_000) => (4, 2),
        (VoltageScale::Scale1, ..=225_000_000) => (5, 2),
        // VOS 2 range VCORE 1.05V - 1.15V
        (VoltageScale::Scale2, ..=34) => (0, 0),
        (VoltageScale::Scale2, ..=68) => (1, 0),
        (VoltageScale::Scale2, ..=102) => (2, 1),
        (VoltageScale::Scale2, ..=136) => (3, 1),
        (VoltageScale::Scale2, ..=160) => (4, 2),
        // VOS 3 range VCORE 0.95V - 1.05V
        (VoltageScale::Scale3, ..=22) => (0, 0),
        (VoltageScale::Scale3, ..=44) => (1, 0),
        (VoltageScale::Scale3, ..=66) => (2, 1),
        (VoltageScale::Scale3, ..=88) => (3, 1),
        _ => unreachable!(),
    };

    debug!("flash: latency={} wrhighfreq={}", latency, wrhighfreq);

    FLASH.acr().write(|w| {
        w.set_wrhighfreq(wrhighfreq);
        w.set_latency(latency);
    });
    while FLASH.acr().read().latency() != latency {}
}
