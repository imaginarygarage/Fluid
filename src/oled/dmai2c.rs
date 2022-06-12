use core::{cmp, mem, cell::RefCell};
use cortex_m;
use cortex_m::interrupt::Mutex;
use stm32f0xx_hal::pac::{interrupt, Interrupt, I2C1, DMA1};


// Global variables for the DMA tx complete interrupt
static DMA_I2C: Mutex<RefCell<Option<DMAi2c>>> = Mutex::new(RefCell::new(None));
static DMA_I2C_BUFFER: Mutex<RefCell<Option<I2CBuffer>>> = Mutex::new(RefCell::new(None));


/// A buffer for I2C transmissions. If the length of the buffer
/// is greater than the tx_size, data will be transmitted in
/// tx_size increments.
#[derive(Copy, Clone)]
pub struct I2CBuffer {
    pub data: &'static [u8],
    pub tx_size: u8,
}


/// An interface for DMA I2C transmissions
pub struct DMAi2c {
    i2c: I2C1,
    dma: DMA1,
    tx_data: Option<I2CBuffer>,
    tx_index: usize,
}

impl DMAi2c {
    /// Initialize the DMAi2c interface.
    /// TODO: consider generalizing beyond I2C1 and DMA1,
    ///       or at least not taking all DMA channels.
    pub fn init(mut i2c: I2C1, mut dma: DMA1) {
        // configure the I2C and DMA peripherals
        DMAi2c::init_i2c(&mut i2c);
        DMAi2c::init_dma(&mut dma);

        // Create the DMAi2c struct
        let dma_i2c = DMAi2c {
            i2c,
            dma,
            tx_data: None,
            tx_index: 0,
        };

        // move the DMAi2c struct to a global mutex
        // for consumption by the DMA interrupt.
        DMAi2c::give_interface(dma_i2c);
    }

    /// Transmit some data. This blocks until tx is possible
    pub fn tx(data: &'static [u8], tx_size: Option<usize>) {
        while DMAi2c::tx_in_progress() {
            // Wait until pending buffer is available
        }

        // Get the tx_size
        let tx_size = match tx_size {
            Some(x) => x,
            None => data.len(),
        } as u8;

        //Move data ref to global mutex for DMA interrupt
        DMAi2c::set_tx_buffer(data, tx_size);

        // trigger the DMA interrupt to begin tx
        cortex_m::peripheral::NVIC::pend(Interrupt::DMA1_CH2_3);
    }

    /// Determine if a transmission is in progress.
    /// The DMA Interrupt takes the DMAi2c interface while
    /// transmitting, so if it resides in the global mutex,
    /// no transmission is in progress.
    pub fn tx_in_progress() -> bool {
        let mut in_progress = true;
        cortex_m::interrupt::free(|cs| {
            if DMA_I2C.borrow(cs).borrow().is_some() {
                in_progress = false;
            }
        });
        in_progress
    }

    // Transmit a string of bytes of the given length, 
    // starting at the given address. 
    // Note: only called from DMA interrupt
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
        // TODO: move "slave" address to be a tx parameter
        self.i2c.cr2.modify(|_, w| w.sadd().bits(0b01111000)
                                    .nbytes().bits(length as u8)
                                    .autoend().set_bit()
                                    .rd_wrn().clear_bit()
                                    .start().set_bit());

    }

    // Return the interface to the global mutex
    fn give_interface(intf: DMAi2c) {
        Self::swap_interface(&mut Some(intf));
    }

    // Swap an interface with the global value
    // Note: If some interface is acquired, 
    //       it must be given back
    fn swap_interface(intf: &mut Option<DMAi2c>) {
        cortex_m::interrupt::free(|cs| {
            mem::swap(intf, &mut DMA_I2C.borrow(cs).borrow_mut());
        });
    }

    // Take the interface if it's available
    // Note: must be given back!
    fn take_interface() -> Option<DMAi2c> {
        let mut intf = None;
        Self::swap_interface(&mut intf);
        intf
    }

    // Set the tx buffer data in the global mutex
    fn set_tx_buffer(data: &'static [u8], tx_size: u8) {
        Self::swap_tx_buffer(&mut Some(I2CBuffer{data, tx_size}));
    }

    // swap a tx buffer data with the global value
    fn swap_tx_buffer(data: &mut Option<I2CBuffer>) {
        cortex_m::interrupt::free(|cs| {
            mem::swap(data, &mut DMA_I2C_BUFFER.borrow(cs).borrow_mut());
        });
    }

    // Initialize the DMA peripheral for I2C transmissions
    fn init_dma(dma: &mut DMA1) {
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
            cortex_m::peripheral::NVIC::unmask(Interrupt::DMA1_CH2_3);
        }
    }

    // Initialize the I2C peripheral for DMA transmissions
    fn init_i2c(i2c: &mut I2C1) {
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
fn DMA1_CH2_3() {
    // DMA I2C interface
    static mut I2C_INTERFACE: Option<DMAi2c> = None;

    // Take the DMA I2C interface if not already owned
    if I2C_INTERFACE.is_none() {
        *I2C_INTERFACE = DMAi2c::take_interface();
    }

    let mut tx_complete = false;
    if let Some(i2c) = I2C_INTERFACE {
        // clear interrupt flag
        i2c.dma.ifcr.write(|w| w.ctcif2().set_bit());

        // Get the data if not already acquired
        if i2c.tx_data.is_none() {
            DMAi2c::swap_tx_buffer(&mut i2c.tx_data);
        }

        // TX any untransmitted data
        match i2c.tx_data {
            Some(tx_data) if i2c.tx_index < tx_data.data.len() => {
                // TX next block of data
                let transmission_address = tx_data.data.as_ptr() as u32 + i2c.tx_index as u32;
                let transmission_length = cmp::min(tx_data.data.len() - i2c.tx_index, tx_data.tx_size as usize) as u8;
                i2c.tx_data_addr_len(transmission_address, transmission_length);
                i2c.tx_index += transmission_length as usize;
            },
            _ => { 
                // TX complete, reset the tx data
                tx_complete = true;
                i2c.tx_data = None;
                i2c.tx_index = 0;
            },
        }
    }

    // When the transmission is complete, return the DMA I2C interface
    if tx_complete {
        DMAi2c::swap_interface(I2C_INTERFACE);
    }
}
