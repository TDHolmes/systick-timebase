//! Provider of a timebase based on [`systick`](cortex_m::peripheral::SYST).
//!
//! This timebase depends on the [`SYST`] hardware common to most cortex-M devices.
//! The timebases' configured resolution is the same as the core clock. Depending on the speed
//! of the source clock provided to [`SYST`], this timebase might quickly overflow and be useless.
//! To mitigate this, one can use the `extended` feature, which extends the resolution of
//! the counter from 24 bit to [`u32`] or [`u64`] (depending on `container-u64`) using the
//! [`SysTick`] exception. It is set to expire just before overflow, so you can expect an exception
//! to fire every 2**24 clock cycles.
//!
//! ```no_run
//! # macro_rules! log {($s:expr, $($tt:tt)*) => { () }}
//! use systick_timebase::{SysTickTimebase, SystClkSource};
//! use cortex_m::Peripherals as CorePeripherals;
//!
//! const FREQ: u32 = 24_000_000; // if our core clock is 24 MHz
//!
//! let core = CorePeripherals::take().unwrap();
//! let timebase = cortex_m::singleton!(
//!     : SysTickTimebase::<FREQ> = SysTickTimebase::new(
//!         core.SYST,
//!         SystClkSource::Core,
//!         FREQ,
//!     )
//! );
//!
//! for i in 0..100 {
//!     log!("time: {}", timebase.time());
//! }
//! ```
//!
//! ## Features
//!
//! ### `extended`
//!
//! as discussed above, extend the native resolution of 24 bits to either 32 or 64 bits
//! using the [`SysTick`] exception. The exception fires ever 2**24 clock cycles.
//!
//! ### `container-u64`
//!
//! enables the return type to be `u64` instead of `u32`.
//!
//! [`SYST`]: cortex_m::peripheral::SYST
//! [`SysTick`]: `cortex_m::peripheral::scb::Exception::SysTick`
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "extended")]
use atomic_polyfill::{AtomicU32, Ordering};
pub use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
#[cfg(feature = "embedded-hal")]
use embedded_hal::blocking::delay::{DelayMs, DelayUs};

/// The container we return when reading out the timebase.
#[cfg(feature = "container-u64")]
pub type TBContainer = u64;
#[cfg(not(feature = "container-u64"))]
pub type TBContainer = u32;

/// Our instant type
pub type TBInstant<const FREQ: u32> = fugit::Instant<TBContainer, 1, FREQ>;

#[cfg(feature = "extended")]
/// Tracker of `systick` cycle count overflows to extend systick's 24 bit timer
static ROLLOVER_COUNT: AtomicU32 = AtomicU32::new(0);

/// The reload value of the [`systick`](cortex_m::peripheral::SYST) peripheral. Also is the max it can go (2**24).
const SYSTICK_RELOAD: u32 = 0x00FF_FFFF;
/// the resolution of [`systick`](cortex_m::peripheral::SYST), 2**24
#[cfg(feature = "extended")]
const SYSTICK_RESOLUTION: TBContainer = 0x0100_0000;

/// [`systick`](cortex_m::peripheral::SYST) timebase.
///
/// The frequency of the [`systick`](cortex_m::peripheral::SYST) is encoded using the parameter `FREQ`.
pub struct SysTickTimebase<const FREQ: u32> {
    #[allow(unused)]
    /// we currently take SYST by value only to ensure ownership
    systick: SYST,
    /// Begrudgingly take the clock frequency by value as well for when we can't use generics
    #[allow(unused)]
    sysclk: u32,
}

impl<const FREQ: u32> SysTickTimebase<FREQ> {
    /// Enable the [`systick`](cortex_m::peripheral::SYST) and provide a new [`SysTickTimebase`].
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    ///
    /// # Panics
    /// asserts that the compile time constant `FREQ` matches the runtime provided `sysclk`
    #[must_use]
    pub fn new(mut systick: SYST, clock_source: SystClkSource, sysclk: u32) -> Self {
        assert!(FREQ == sysclk);

        systick.disable_counter();
        systick.set_clock_source(clock_source);
        systick.clear_current();
        systick.set_reload(SYSTICK_RELOAD);
        systick.enable_counter();

        #[cfg(feature = "extended")]
        systick.enable_interrupt();

        Self { systick, sysclk }
    }

    /// Reads the current value from [`systick`](cortex_m::peripheral::SYST).
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn read(&self) -> TBInstant<FREQ> {
        // Read SYSTICK and maybe account for rollovers
        let ticks = {
            #[cfg(feature = "extended")]
            {
                // read the clock & ROLLOVER_COUNT. We read `SYST` twice because we need to detect
                // if we've rolled over, and if we have make sure we have the right value for ROLLOVER_COUNT.
                let first = SYST::get_current();
                let rollover_count: TBContainer = ROLLOVER_COUNT.load(Ordering::Acquire).into();
                let second = SYST::get_current();

                // Since the SYSTICK counter is a count down timer, check if first is larger than second
                if first > second {
                    // The usual case. We did not roll over between the first and second reading,
                    // and because of that we also know we got a valid read on ROLLOVER_COUNT.
                    rollover_count * SYSTICK_RESOLUTION + TBContainer::from(SYSTICK_RELOAD - first)
                } else {
                    // we rolled over sometime between the first and second read. We may or may not have
                    // caught the right ROLLOVER_COUNT, so grab that again and then use the second reading.
                    let rollover_count: TBContainer = ROLLOVER_COUNT.load(Ordering::Acquire).into();
                    rollover_count * SYSTICK_RESOLUTION + TBContainer::from(SYSTICK_RELOAD - second)
                }
            }

            #[cfg(not(feature = "extended"))]
            {
                // We aren't trying to be fancy here, we don't care if this rolled over from the last read.
                TBContainer::from(SYSTICK_RELOAD - SYST::get_current())
            }
        };

        TBInstant::<FREQ>::from_ticks(ticks)
    }
}

#[cfg(all(feature = "embedded-hal", feature = "container-u64"))]
impl<const FREQ: u32> DelayUs<u64> for SysTickTimebase<FREQ> {
    fn delay_us(&mut self, us: u64) {
        let ticks_per_us: u64 = self.sysclk as u64 / 1_000_000;

        let start = self.read().ticks();
        let end = start + ticks_per_us * us;
        let mut previous = start;
        loop {
            let time = self.read().ticks();
            if time >= end {
                break;
            }
            if time < previous {
                panic!("Detected overflow while delaying");
            }

            previous = time;
        }
    }
}

#[cfg(all(feature = "embedded-hal", feature = "container-u64"))]
impl<const FREQ: u32> DelayMs<u64> for SysTickTimebase<FREQ> {
    fn delay_ms(&mut self, ms: u64) {
        self.delay_us(ms * 1_000);
    }
}

#[cfg(all(feature = "embedded-hal", not(feature = "container-u64")))]
impl<const FREQ: u32> DelayUs<u32> for SysTickTimebase<FREQ> {
    fn delay_us(&mut self, us: u32) {
        let ticks_per_us: u32 = self.sysclk as u32 / 1_000_000;

        let start = self.read().ticks();
        let end = start + ticks_per_us * us;
        let mut previous = start;
        loop {
            let time = self.read().ticks();
            if time >= end {
                break;
            }
            if time < previous {
                panic!("Detected overflow while delaying");
            }

            previous = time;
        }
    }
}

#[cfg(all(feature = "embedded-hal", not(feature = "container-u64")))]
impl<const FREQ: u32> DelayMs<u32> for SysTickTimebase<FREQ> {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }
}

#[cfg(feature = "embedded-hal")]
macro_rules! impl_delay_us {
    ($($T:ty),+) => {
        $(#[cfg(all(feature = "embedded-hal", not(feature = "container-u64")))]
        impl<const FREQ: u32> DelayUs<$T> for SysTickTimebase<FREQ> {
            fn delay_us(&mut self, us: $T) {
                self.delay_us(us as u32);
            }
        }

        #[cfg(all(feature = "embedded-hal", feature = "container-u64"))]
        impl<const FREQ: u32> DelayUs<$T> for SysTickTimebase<FREQ> {
            fn delay_us(&mut self, us: $T) {
                self.delay_us(us as u64);
            }
        }

        #[cfg(all(feature = "embedded-hal", not(feature = "container-u64")))]
        impl<const FREQ: u32> DelayMs<$T> for SysTickTimebase<FREQ> {
            fn delay_ms(&mut self, ms: $T) {
                self.delay_us(ms as u32 * 1_000);
            }
        }

        #[cfg(all(feature = "embedded-hal", feature = "container-u64"))]
        impl<const FREQ: u32> DelayMs<$T> for SysTickTimebase<FREQ> {
            fn delay_ms(&mut self, ms: $T) {
                self.delay_us(ms as u64 * 1_000);
            }
        })+
    };
}

#[cfg(all(feature = "embedded-hal", feature = "container-u64"))]
impl_delay_us!(u8, u16, u32);

#[cfg(all(feature = "embedded-hal", not(feature = "container-u64")))]
impl_delay_us!(u8, u16);

#[cfg(feature = "extended")]
use cortex_m_rt::exception;

#[cfg(feature = "extended")]
#[exception]
#[allow(non_snake_case)]
fn SysTick() {
    ROLLOVER_COUNT.fetch_add(1, Ordering::Release);
}
