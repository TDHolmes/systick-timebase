//! Prints "Hello, world!" on the host console using semihosting

#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m::Peripherals as CorePeripherals;
use cortex_m_rt::entry;

const FREQ: u32 = 12_000_000;
static mut SYSTICK: Option<systick_timebase::SysTickTimebase<FREQ>> = None;

type Duration = fugit::Duration<systick_timebase::TBContainer, 1, FREQ>;

fn get_systick() -> &'static systick_timebase::SysTickTimebase<FREQ> {
    unsafe { SYSTICK.as_ref().expect("systick uninitialized") }
}

macro_rules! log {
    () => {
        let ticks_us = Duration::micros(get_systick().read().ticks()).ticks();
        ::cortex_m_semihosting::hprintln!("{} µs: ", ticks_us)
    };
    ($s:expr) => {
        let ticks_us = Duration::micros(get_systick().read().ticks()).ticks();
        ::cortex_m_semihosting::hprintln!(concat!("{} µs: ", $s), ticks_us)
    };
    ($s:expr, $($tt:tt)*) => {
        let ticks_us = Duration::micros(get_systick().read().ticks()).ticks();
        ::cortex_m_semihosting::hprintln!(concat!("{} µs: ", $s), ticks_us, $($tt)*)
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
        log!("Hello, world! {}", i);
    }

    // exit QEMU
    // NOTE do not run this on hardware; it can corrupt OpenOCD state
    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);

    loop {}
}
