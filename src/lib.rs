//! Provider of a timebase based on [`systick`](cortex_m::peripheral::SYST).
//!
//! This timebase depends on the [`SYST`] hardware common to most cortex-M devices.
//! The timebases' configured resolution is the same as the core clock. Depending on the speed
//! if the source clock provided to [`SYST`], this timebase might quickly overflow and be useless.
//! To mitigate this, one can use the `extended` feature, which extends the resolution of
//! the counter from 24 bit to [`u32`] or [`u64`] using the [`SysTick`] exception. It is set
//! to expire just before overflow, so you can expect an exception to fire every 2**24
//! clock cycles.
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

use cortex_m::peripheral::{syst::SystClkSource, SYST};

#[cfg(feature = "extended")]
use core::sync::atomic::{AtomicU32, Ordering};

/// The container we return when reading out the timebase.
#[cfg(feature = "container-u64")]
pub type TBContainer = u64;
#[cfg(not(feature = "container-u64"))]
pub type TBContainer = u32;

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
    // we currently take SYST by value only to ensure ownership
    systick: SYST,
}

impl<const FREQ: u32> SysTickTimebase<FREQ> {
    /// Enable the [`systick`](cortex_m::peripheral::SYST) and provide a new [`SysTickTimebase`].
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    ///
    /// # Panics
    /// asserts that the compile time constant `FREQ` matches the runtime provided `sysclk`
    pub fn new(mut systick: SYST, clock_source: SystClkSource, sysclk: u32) -> Self {
        assert!(FREQ == sysclk);

        systick.disable_counter();
        systick.set_clock_source(clock_source);
        systick.clear_current();
        systick.set_reload(SYSTICK_RELOAD);
        systick.enable_counter();

        #[cfg(feature = "extended")]
        systick.enable_interrupt();

        Self { systick }
    }

    /// Reads the current value from [`systick`](cortex_m::peripheral::SYST).
    pub fn read(&self) -> fugit::Instant<TBContainer, 1, FREQ> {
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

        fugit::Instant::<TBContainer, 1, FREQ>::from_ticks(ticks)
    }
}

#[cfg(feature = "extended")]
use cortex_m_rt::exception;

#[cfg(feature = "extended")]
#[exception]
#[allow(non_snake_case)]
fn SysTick() {
    ROLLOVER_COUNT.fetch_add(1, Ordering::Release);
}
