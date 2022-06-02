#![no_std]
#![no_main]

use panic_halt as _;

use stm32f0xx_hal as hal;

use crate::hal::{gpio::*, pac, prelude::*};

use cortex_m::{peripheral::syst::SystClkSource::Core, Peripherals};
use cortex_m_rt::{entry, exception};

mod oled;
use oled::OLEDDriver;

#[entry]
fn main() -> ! {
    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        cortex_m::interrupt::free(move |cs| {
            // configure the clock to a frequency of 48MHz using
            // the internal oscillator multiplied by the PLL
            let mut rcc = p.RCC.configure()
                               .sysclk(48.mhz())
                               .freeze(&mut p.FLASH);

            // Configure and enable the SysTick interrupt for 1ms period
            let mut systick = cp.SYST;
            systick.set_clock_source(Core);
            systick.set_reload(48_000 - 1);
            systick.enable_counter();
            systick.enable_interrupt();

            // Configure pins for I2C
            let gpiob = p.GPIOB.split(&mut rcc);
            let _sda = gpiob.pb7.into_alternate_af1(cs);
            let _scl = gpiob.pb6.into_alternate_af1(cs);

            // Create the display driver and set some pixels
            let mut display = OLEDDriver::new(p.I2C1, p.DMA1, &mut rcc);
            display.set_pixel(0, 0, true);
            display.set_pixel(127, 63, true);
            display.invert();
            display.tx_frame();
        });
    }

    loop {
        continue;
    }
}

// Define an exception handler, i.e. function to call when exception occurs. Here, if our SysTick
// timer generates an exception the following handler will be called
#[exception]
fn SysTick() {
    // Do some stuff
}
