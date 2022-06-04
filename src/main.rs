#![no_std]
#![no_main]

use panic_halt as _;

use stm32f0xx_hal as hal;

use crate::hal::{gpio::*, pac, prelude::*};

use cortex_m::{peripheral::syst::SystClkSource::Core, Peripherals};
use cortex_m_rt::{entry, exception};

mod oled;
use oled::OLEDDriver;

mod fluid;

#[entry]
fn main() -> ! {
    if let (Some(mut p), Some(cp)) = (pac::Peripherals::take(), Peripherals::take()) {
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
        cortex_m::interrupt::free(move |cs| {
            let _sda = gpiob.pb7.into_alternate_af1(cs);
            let _scl = gpiob.pb6.into_alternate_af1(cs);
        });

        // Create the display driver and set some pixels
        let mut display = OLEDDriver::new(p.I2C1, p.DMA1, &mut rcc);
        let mut fluid_sim = fluid::Fluid::<75>::new(125, 61);
        loop {
            fluid_sim.step();
            for particle in fluid_sim.get_particles() {
                let (x, y) = particle.get_display_position();
                draw_particle(&mut display, x as usize, y as usize)
            }
            display.tx_frame();
        }
    }

    loop {
        //FAIL!
        continue;
    }
}

fn draw_particle(display: &mut OLEDDriver, x: usize, y: usize) {
    const PIXELS: [(usize,usize); 12] = [
                (1, 0), (2, 0),
        (0, 1), (1, 1), (2, 1), (3, 1),
        (0, 2), (1, 2), (2, 2), (3, 2),
                (1, 3), (2, 3),
    ];

    for (dx, dy) in PIXELS {
        display.set_pixel(x + dx, y + dy, true);
    }
}

// Define an exception handler, i.e. function to call when exception occurs. Here, if our SysTick
// timer generates an exception the following handler will be called
#[exception]
fn SysTick() {
    // Do some stuff
}
