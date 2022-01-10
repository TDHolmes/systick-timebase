//! Prints "Hello, world!" on the host console using semihosting

#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m::Peripherals as CorePeripherals;
use cortex_m_rt::entry;
use systick_timebase::{SysTickTimebase, TBContainer, TBInstant};

const FREQ: u32 = 12_000_000;
static mut SYSTICK: Option<SysTickTimebase<FREQ>> = None;

type Duration = fugit::Duration<TBContainer, 1, FREQ>;

/// Gets a reference to the global systick timebase.
fn get_systick() -> &'static SysTickTimebase<FREQ> {
    unsafe { SYSTICK.as_ref().expect("systick uninitialized") }
}

/// Converts the given instant to time in µs.
fn to_micros(instant: TBInstant<FREQ>) -> TBContainer {
    Duration::micros(instant.ticks()).ticks()
}

macro_rules! log {
    () => {
        ::cortex_m_semihosting::hprintln!(
            "{} µs: ",
            to_micros(get_systick().read())
        )
    };
    ($s:expr) => {
        ::cortex_m_semihosting::hprintln!(
            concat!("{} µs: ", $s),
            to_micros(get_systick().read())
        )
    };
    ($s:expr, $($tt:tt)*) => {
        ::cortex_m_semihosting::hprintln!(
            concat!("{} µs: ", $s),
            to_micros(get_systick().read()),
            $($tt)*
        )
    };
}

#[entry]
fn main() -> ! {
    let core = CorePeripherals::take().unwrap();
    unsafe {
        SYSTICK = Some(systick_timebase::SysTickTimebase::new(
            core.SYST,
            systick_timebase::SystClkSource::Core,
            FREQ,
        ));
    }

    for i in 0..100 {
        log!("Hello, world! {}", i).ok();
    }

    // exit QEMU
    // NOTE do not run this on hardware; it can corrupt OpenOCD state
    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);

    loop {}
}
