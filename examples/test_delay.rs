//! Checks for rollover conditions at 2**24 and 2**32, depending on features.

#![no_main]
#![no_std]

use cortex_m::Peripherals as CorePeripherals;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use panic_halt as _;

const FREQ: u32 = 12_000_000;

static mut SYSTICK: Option<systick_timebase::SysTickTimebase<FREQ>> = None;

/// Gets a reference to the global systick timebase.
fn get_systick_mut() -> &'static mut systick_timebase::SysTickTimebase<FREQ> {
    unsafe { SYSTICK.as_mut().expect("systick uninitialized") }
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

    hprintln!("250us delay (u8)").ok();
    get_systick_mut().delay_us(250_u8);
    hprintln!("250us delay (u16)").ok();
    get_systick_mut().delay_us(250_u16);
    hprintln!("250us delay (u32)").ok();
    get_systick_mut().delay_us(250_u32);
    #[cfg(feature = "container-u64")]
    {
        hprintln!("250us delay (u64)").ok();
        get_systick_mut().delay_us(250_u64);
    }

    hprintln!("250ms delay (u8)").ok();
    get_systick_mut().delay_ms(250_u8);
    hprintln!("250ms delay (u16)").ok();
    get_systick_mut().delay_ms(250_u16);
    hprintln!("250ms delay (u32)").ok();
    get_systick_mut().delay_ms(250_u32);
    #[cfg(feature = "container-u64")]
    {
        hprintln!("250ms delay (u64)").ok();
        get_systick_mut().delay_ms(250_u64);
    }

    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}
