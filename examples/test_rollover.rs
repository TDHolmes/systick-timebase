//! Checks for rollover conditions at 2**24 and 2**32, depending on features.

#![no_main]
#![no_std]

use cortex_m::Peripherals as CorePeripherals;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use panic_halt as _;

const FREQ: u32 = 12_000_000;

static mut SYSTICK: Option<systick_timebase::SysTickTimebase<FREQ>> = None;

/// Gets a reference to the global systick timebase.
fn get_systick() -> &'static systick_timebase::SysTickTimebase<FREQ> {
    unsafe { SYSTICK.as_ref().expect("systick uninitialized") }
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
    let mut previous_time = get_systick().read();

    #[cfg(all(feature = "extended", not(feature = "container-u64")))]
    {
        let mut cnt: usize = 0;
        hprintln!("Extended mode, with u32 container. Look for rollover [2**24, 2**32)").ok();
        loop {
            let time = get_systick().read();
            if time < previous_time {
                // rollover was seen. Check if it's greater than 2**24
                if previous_time.ticks() > (2 << 24) {
                    hprintln!("Rollover seen in the expected place").ok();
                    debug::exit(debug::EXIT_SUCCESS);
                    break;
                } else {
                    hprintln!(
                        "Unexpected rollover: {:08X} -> {:08X}",
                        previous_time.ticks(),
                        time.ticks()
                    )
                    .ok();
                    debug::exit(debug::EXIT_FAILURE);
                    break;
                }
            }

            cnt += 1;
            previous_time = time;

            if cnt == 25 {
                cnt = 0;
                hprintln!("time: {:08X}", time.ticks()).ok();
            }
        }
    }

    #[cfg(all(feature = "extended", feature = "container-u64"))]
    {
        hprintln!("Extended mode, with u64 container. Look for time > 2**32").ok();
        loop {
            // we are running in extended mode, but not u64. We should never see rollover < 2**64
            let time = get_systick().read();
            if time < previous_time {
                hprintln!(
                    "Rollover! {:08X} -> {:08X}",
                    previous_time.ticks(),
                    time.ticks()
                )
                .ok();
                debug::exit(debug::EXIT_FAILURE);
                break;
            }

            if time.ticks() > u32::MAX as u64 {
                hprintln!("Time seen past 2**32").ok();
                debug::exit(debug::EXIT_SUCCESS);
                break;
            }

            previous_time = time;
        }
    }

    #[cfg(not(feature = "extended"))]
    {
        hprintln!("Regular systick with no extension. Look for rollover around 2**24").ok();
        loop {
            // we are running in extended mode, but not u64. We should see rollover around 2**32
            let time = get_systick().read();

            if time.ticks() >= (2 << 24) {
                hprintln!("Time seen past 2**24").ok();
                debug::exit(debug::EXIT_FAILURE);
                break;
            }

            if time < previous_time {
                hprintln!(
                    "Rollover! {:08X} -> {:08X}",
                    previous_time.ticks(),
                    time.ticks()
                )
                .ok();
                debug::exit(debug::EXIT_SUCCESS);
                break;
            }

            previous_time = time;
        }
    }

    loop {}
}
