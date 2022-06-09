use core::cell::RefCell;
use core::mem;

use cortex_m;
use cortex_m::interrupt::Mutex;

use stm32f0xx_hal::pac::{interrupt, Interrupt, I2C1, DMA1};
use stm32f0xx_hal::rcc::Rcc;


static DMA_I2C: Mutex<RefCell<Option<DMAi2c>>> = Mutex::new(RefCell::new(None));
static DMA_I2C_BUF_REF: Mutex<RefCell<Option<&'static I2CBuffer>>> = Mutex::new(RefCell::new(None));


/// A buffer for I2C transmissions. If the length of the buffer
/// is greater than the tx_size, data will be transmitted in
/// tx_size increments.
pub struct I2CBuffer {
    pub data: &'static mut [u8],
    pub tx_size: u8,
}


/// An interface for DMA I2C transmissions
pub struct DMAi2c {
    i2c: I2C1,
    dma: DMA1,
    tx_data: Option<&'static I2CBuffer>,
    tx_index: usize,
}

impl DMAi2c {
    /// Initialize the DMAi2c interface.
    /// TODO: consider generalizing beyond I2C1 and DMA1,
    ///       or at least not taking all DMA channels.
    pub fn init(mut i2c: I2C1, mut dma: DMA1, rcc: &mut Rcc) {
        // configure the I2C and DMA peripherals
        DMAi2c::init_i2c(&mut i2c, rcc);
        DMAi2c::init_dma(&mut dma, rcc);

        // Create the DMAi2c struct
        let dma_i2c = DMAi2c {
            i2c,
            dma,
            tx_data: None,
            tx_index: 0,
        };

        // move the DMAi2c struct to a global mutex
        // for consumption by the DMA interrupt.
        cortex_m::interrupt::free(|cs| {
            let _old_value_is_none = DMA_I2C.borrow(cs).replace(Some(dma_i2c));
        });
    }

    /// Transmit some data. This blocks until tx is possible
    pub fn tx(data: &'static I2CBuffer) {
        // Wait until pending buffer is available
        let mut count = 2;
        while DMAi2c::tx_in_progress() {
            // TODO: consider a more sophisticated delay
            for i in 0..count {
                count += -1 + 2 * (i % 2);
            }
            if count > 1_000_000_000 {
                return;
            }
            count += 2;

        }

        //Move data ref to global mutex for DMA interrupt
        cortex_m::interrupt::free(|cs| {
            let _old_value = DMA_I2C_BUF_REF.borrow(cs).replace(Some(data));
        });

        // trigger the DMA interrupt to begin tx
        //cortex_m::peripheral::NVIC::pend(Interrupt::DMA1_CH2_3);
        cortex_m::peripheral::NVIC::pend(Interrupt::DMA1_CH2_3_DMA2_CH1_2);
    }

    /// Determine if a transmission is in progress.
    /// The DMA Interrupt takes the DMAi2c interface while
    /// transmitting, so if it resides in the global mutex,
    /// no transmission is in progress.
    fn tx_in_progress() -> bool {
        let mut in_progress = true;
        cortex_m::interrupt::free(|cs| {
            if DMA_I2C.borrow(cs).borrow().is_some() {
                in_progress = false;
            }
        });
        in_progress
    }

    // only call this from DMA interrupt
    fn tx_data_addr_len(&mut self, address: u32, length: u8) {
        // disable DMA peripheral while updating configuration
        self.dma.ch2.cr.modify(|_, w| w.en().disabled());
        while self.dma.ch2.cr.read().en().is_enabled() {
            // wait for DMA to be disabled
        }

        // set the start address for the DMA transfer
        self.dma.ch2.mar.write(|w| unsafe { w.bits(address) });

        // set the number of bytes to be transfered
        self.dma.ch2.ndtr.write(|w| unsafe { w.bits(length as u32) });

        // enable the DMA peripheral
        self.dma.ch2.cr.modify(|_, w| w.en().enabled());

        // ensure I2C is not mid transfer
        while self.i2c.isr.read().txe().is_not_empty() {
            // wait for I2C transmit data register to be empty
        }

        // configure the I2C peripheral for the transfer and start
        // TODO: move OLED address to constants
        self.i2c.cr2.modify(|_, w| w.sadd().bits(0b01111000)
                                    .nbytes().bits(length as u8)
                                    .autoend().set_bit()
                                    .rd_wrn().clear_bit()
                                    .start().set_bit());

    }

    fn init_dma(dma: &mut DMA1, rcc: &mut Rcc) {
        // Enable clock to the DMA peripheral
        // FIXME: I had to modify stm32f0xx_hal/src/rcc.rs to allow access to regs
        //        there must be a better way.
        rcc.regs.ahbenr.modify(|_, w| w.dmaen().enabled());

        // configure DMA1 channel 2 for I2C transmissions
        dma.ch2.cr.modify(|_, w| w.mem2mem().disabled()
                                  .pl().very_high()
                                  .msize().bits8()
                                  .psize().bits8()
                                  .minc().enabled()
                                  .pinc().disabled()
                                  .circ().disabled()
                                  .dir().from_memory()
                                  .teie().disabled()
                                  .htie().disabled()
                                  .tcie().enabled());

        // set peripheral address register to I2C1_TXDR
        dma.ch2.par.write(|w| unsafe { w.bits(0x4000_5428) });

        // enable the dma peripheral
        dma.ch2.cr.modify(|_, w| w.en().enabled());

        // unmask the DMA transfer interrupt
        unsafe {
            //cortex_m::peripheral::NVIC::unmask(Interrupt::DMA1_CH2_3);
            cortex_m::peripheral::NVIC::unmask(Interrupt::DMA1_CH2_3_DMA2_CH1_2);
        }
    }

    fn init_i2c(i2c: &mut I2C1, rcc: &mut Rcc) {
        // initialize I2C clocks
        //  - set the system clock as the I2C1 clock source
        //  - enable clock to I2C1
        //  FIXME: I had to modify stm32f0xx_hal/src/rcc.rs to allow access to regs
        //         there must be a better way.
        rcc.regs.cfgr3.modify(|_, w| w.i2c1sw().sysclk());
        rcc.regs.apb1enr.modify(|_, w| w.i2c1en().enabled());

        // ensure i2c peripheral is disabled while changing configuration
        i2c.cr1.write(|w| w.pe().disabled());
        while i2c.cr1.read().pe().is_enabled() {
            // wait for i2c to be disabled
        }

        // update the timing register for 400kHZ operation
        i2c.timingr.write(|w| w.scll().bits(26)   // SCL low period
                               .sclh().bits(20)   // SCL high period
                               .sdadel().bits(0)  // SDA delay
                               .scldel().bits(9)  // SCL delay
                               .presc().bits(1)); // clock prescaler

        // enable DMA transmission requests and start the I2C peripheral
        i2c.cr1.write(|w| w.txdmaen().enabled()
                           .pe().enabled());
    }
}


#[interrupt]
fn DMA1_CH2_3_DMA2_CH1_2() {
    // DMA I2C interface
    static mut dma_i2c: Option<DMAi2c> = None;

    //TODO:
    // if((DMA1 -> ISR) & DMA_ISR_TCIF2)	//only if the channel 2 transfer complete flag is set
    //cortex_m::peripheral::NVIC::unpend(Interrupt::DMA1_CH2_3);

    // Take the DMA I2C interface if not already owned
    if dma_i2c.is_none() {
        cortex_m::interrupt::free(|cs| {
            mem::swap(dma_i2c, &mut DMA_I2C.borrow(cs).borrow_mut());
        });
    }

    let mut tx_complete = false;
    if let Some(i2c) = dma_i2c {
        //TODO: move below to an I2C tx function
        
        // clear interrupt flag
        i2c.dma.ifcr.write(|w| w.ctcif2().set_bit());

        // Get the data if not already acquired
        if i2c.tx_data.is_none() {
            cortex_m::interrupt::free(|cs| {
                mem::swap(&mut i2c.tx_data, &mut DMA_I2C_BUF_REF.borrow(cs).borrow_mut());
            });
        }

        // TX any untransmitted data
        if let Some(tx_data) = i2c.tx_data {
            if i2c.tx_index < tx_data.data.len() {
                let transmission_address = tx_data.data.as_ptr() as u32 + i2c.tx_index as u32;
                let transmission_length = core::cmp::min(tx_data.data.len() - i2c.tx_index, tx_data.tx_size as usize) as u8;
                i2c.tx_data_addr_len(transmission_address, transmission_length);
                i2c.tx_index += transmission_length as usize;
            }
            else {
                tx_complete = true;
            }
        }
        else {
            tx_complete = true;
        }

        // Reset the tx data if the transmission is complete
        if tx_complete {
            i2c.tx_data = None;
            i2c.tx_index = 0;
        }
    }

    // When the transmission is complete, return the DMA I2C interface
    if tx_complete {
        cortex_m::interrupt::free(|cs| {
            mem::swap(dma_i2c, &mut DMA_I2C.borrow(cs).borrow_mut());
        });
    }
}
