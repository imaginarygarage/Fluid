#![no_std]
#![no_main]


use panic_halt as _;

use cortex_m::{peripheral::syst::SystClkSource, Peripherals as CorePeripherals};
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::syscall;
use stm32f0xx_hal::{prelude::*, delay::Delay, pac::Peripherals as F0Peripherals};

mod oled;
use oled::OLEDDriver;

mod fluid;
use fluid::Fluid;


#[entry]
fn main() -> ! {
    if let (Some(mut p), Some(cp)) = (F0Peripherals::take(), CorePeripherals::take()) {
        // configure the clock to a frequency of 48MHz using
        // the internal oscillator multiplied by the PLL
        let mut rcc = p.RCC.configure()
                           .sysclk(48.mhz())
                           .freeze(&mut p.FLASH);

        // Configure systick as a delay source
        let mut systick = cp.SYST;
        let mut delay = Delay::new(systick, &rcc);

        // Create the fluid simulation
        let mut fluid_sim = Fluid::<50>::new(125, 61);

        // Configure pins for I2C
        let gpiob = p.GPIOB.split(&mut rcc);
        cortex_m::interrupt::free(move |cs| {
            let _sda = gpiob.pb7.into_alternate_af1(cs);
            let _scl = gpiob.pb6.into_alternate_af1(cs);
        });

        // Initialize and take the OLED display driver
        let mut display = OLEDDriver::new(p.I2C1, p.DMA1, &mut rcc);

        // Transmit the initial frame and delay some amount
        // to allow the user to appreciate the intial state
        draw_particles(&mut display, &fluid_sim);
        display.tx_frame();
        delay.delay_ms(3_000_u16);

        let mut cnt = 0;
        loop {
            // Step the simulation and draw the results
            fluid_sim.step();
            draw_particles(&mut display, &fluid_sim);
            display.tx_frame();

            // Cycle through different gravity configurations 
            // to make the simulation more interesting
            match cnt {
                0..=299 => fluid_sim.set_gravity(0.0, 0.0),
                300..=399 => fluid_sim.set_gravity(0.0, 1.0),
                400..=599 => fluid_sim.set_gravity(1.0, 0.0),
                600..=899 => fluid_sim.set_gravity(-1.0, 0.0),
                900..=999 => fluid_sim.set_gravity(0.0, -1.0),
                1000..=1299 => fluid_sim.set_gravity(0.0, 0.0),
                _ => cnt = 0,
            }
            cnt += 1;
        }
    }

    loop {
        // the loop of shame
        continue;
    }
}


/// Draw an individual particle at the given origin
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

/// Draw all fluid simulation particles
fn draw_particles<const T:usize>(display: &mut OLEDDriver, fluid_sim: &Fluid<T>) {
    for particle in fluid_sim.get_particles() {
        let (x, y) = particle.get_display_position();
        draw_particle(display, x as usize, y as usize);
    }
}


/// Print ASCII string over Semihost
pub fn print(msg: &[u8]) {
    // The file descriptor of stdout on the host
    // Note: STDOUT is not guaranteed to be FD 1
    //       SysCall ISTTY can be used to check
    const STDOUT: usize = 1;

    // The write syscall does not return until complete, 
    // so the lifetime of the reference in msg does not 
    // have to be greater than this function.
    unsafe { 
        syscall!(WRITE, STDOUT, msg.as_ptr(), msg.len());
    };
}

/// Handle the SysTick interrupt
#[exception]
fn SysTick() {
    // Do some stuff
}
