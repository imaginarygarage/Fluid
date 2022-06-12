use stm32f0xx_hal::pac::{I2C1, DMA1};

mod dmai2c;
use dmai2c::DMAi2c;


/// The OLED display used here is a 128 pixel wide by 64 pixel
/// high monochrome display with an SSD1306 controller.
pub const OLED_PXLS_X: usize = 128;
pub const OLED_PXLS_Y: usize = 64;
const OLED_COLS: usize = OLED_PXLS_X;

// An OLED page represents a row of the display 8 pixels tall.
// Each column in this row is represented by a u8 value, where
// a 1 in the LSB represents the top pixel in the on state.
const OLED_PAGES: usize = OLED_PXLS_Y / 8;
const OLED_PAGE_HEADER_SIZE: usize = 7;
const OLED_PAGE_SIZE: usize = OLED_COLS + OLED_PAGE_HEADER_SIZE;
const OLED_FRAME_SIZE: usize = OLED_PAGE_SIZE * OLED_PAGES;


// A list of commands for initializing the OLED display.
static OLED_INIT_CMDS: [&[u8]; 18] = [
    &[0, 0xAE],             //Put OLED display into sleep mode
    &[0, 0x81, 120],        //Set Contrast Value (Command: 0x81) to an 8-bit value ( < 256 )
    &[0, 0xA4],             //Display RAM content (0xA5: all pixels on, regardless of RAM contents)
    &[0, 0xA6],             //Set display ram so that 1 = on (0xA7: inverted)
    &[0, 0x20, 0x00],       //Set Memory Addressing Mode to Horizontal Addressing Mode(00): Column address pointer is incremented by one unless end address has been reached, at which point column address is reset to 0 and page address is incremented by one. Page is reset if already at end address.
    &[0, 0x21, 0x00, 127],  //Set Column start address to 0 and column end address to 127
    &[0, 0x22, 0x00, 0x07], //Set Page start address to 0 and page end address to 7
    &[0, 0x40],             //Set display start line to 0
    &[0, 0xA1],             //Map column address 0 to SEGMENT127 - this way column address zero is on the left side of the screen in my desired orientation
    &[0, 0xA8, 63],         //Set MUX ratio to n + 1. Valid n values: 15-63
    &[0, 0xC0],             //Scan from COM0 to COM[n-1]; n = mux ratio (C8: inverted)
    &[0, 0xD3, 0],          //Set display offset vertical shift to 0
    &[0, 0xDA, 0b00010010], //Set alternative COM pin configuration and disable COM left/right remap
    &[0, 0xD5, 0xC0],       //Set Display Clock Oscillator speed to 12/16, prescaled by 0.
    &[0, 0xD9, 0x22],       //Set pre-charge period to 2 clocks for phase 1, and 2 clocks for phase 2
    &[0, 0xDB, 0x20],       //Set Vcomh deselect level 0x20 ~ 0.77Vcc (0x00: 0.65Vcc; 0x30: 0.83Vcc)*/
    &[0, 0x8D, 0x14],       //Enable Charge Pump when display is on
    &[0, 0xAF],             //Turn on OLED Display
];

// Global mutable OLED buffers
static mut OLED_BUFFER: [u8; OLED_FRAME_SIZE] = [0; OLED_FRAME_SIZE];


pub struct OLEDDriver {
    is_transmitting: bool,
}

impl OLEDDriver {
    /// Create and initialize a new OLED buffer
    pub fn new(i2c: I2C1, dma: DMA1) -> OLEDDriver {
        // Initialize the DMA I2C interface
        DMAi2c::init(i2c, dma);

        // initialize the OLED
        for cmd in &OLED_INIT_CMDS {
            DMAi2c::tx(cmd, None);
        }

        // Initialize the OLED buffer
        unsafe {
            OLEDDriver::init_oled_buffer(&mut OLED_BUFFER);
        }

        // Return the OLED driver
        OLEDDriver {
            is_transmitting: false,
        }
    }

    /// Turn off every pixel
    pub fn clear(&mut self) {
        while self.tx_active() {
            // wait for frame transmission to complete before 
            // modifying display data
        }
        let buffer = self.get_buffer();
        for i in 0..OLED_PAGES {
            let start = i * OLED_PAGE_SIZE + OLED_PAGE_HEADER_SIZE;
            let end = start + OLED_PAGE_SIZE - OLED_PAGE_HEADER_SIZE;
            for byte in &mut buffer[start..end] {
                *byte = 0
            }
        }
    }

    /// Invert the OLED buffer
    pub fn invert(&mut self) {
        while self.tx_active() {
            // wait for frame transmission to complete before 
            // modifying display data
        }
        let buffer = self.get_buffer();
        for i in 0..OLED_PAGES {
            let start = i * OLED_PAGE_SIZE + OLED_PAGE_HEADER_SIZE;
            let end = start + OLED_PAGE_SIZE - OLED_PAGE_HEADER_SIZE;
            for byte in &mut buffer[start..end] {
                *byte = !*byte;
            }
        }
    }

    /// Set a given pixel to be on or off
    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        while self.tx_active() {
            // wait for frame transmission to complete before 
            // modifying display data
        }
        let row = y / 8;
        let bit = y % 8;
        let idx = row * OLED_PAGE_SIZE + OLED_PAGE_HEADER_SIZE + x;
        let buffer = self.get_buffer();
        if on {
            buffer[idx] |= 1 << bit;
        }
        else {
            buffer[idx] &= !(1 << bit);
        }
    }

    /// Transmit the current draw buffer to the OLED.
    /// This also swaps the buffers and clears the new draw buffer.
    pub fn tx_frame(&mut self) {
        DMAi2c::tx(self.get_buffer(), Some(OLED_PAGE_SIZE));
        self.is_transmitting = true;
    }

    fn tx_active(&mut self) -> bool {
        match self.is_transmitting {
            false => false,
            true => {
                self.is_transmitting = DMAi2c::tx_in_progress();
                self.is_transmitting
            }
        }
    }

    /// Initialize an existing OLED buffer
    fn init_oled_buffer(buf: &mut [u8]) {
        // set up buffer page headers
        for i in 0..OLED_PAGES {
            buf[i * OLED_PAGE_SIZE + 0] = 0x80;             // Control byte: specify that the next two bytes will be a command byte followed by another control byte.
            buf[i * OLED_PAGE_SIZE + 1] = 0xB0 + i as u8;   // Command byte: set the page address to page i
            buf[i * OLED_PAGE_SIZE + 2] = 0x80;             // Control byte: specify that the next two bytes will be a command byte followed by another control byte.
            buf[i * OLED_PAGE_SIZE + 3] = 0x10;             // Command byte: set the column address to 0, 1 of 2 (the 4 lsbs in this command correspond to the 4 msbs in the column address)
            buf[i * OLED_PAGE_SIZE + 4] = 0x80;             // Control byte: specify that the next two bytes will be a command byte followed by another control byte.
            buf[i * OLED_PAGE_SIZE + 5] = 0x00;             // Command byte: set the column address to 0, 2 of 2 (the 4 lsbs in this command correspond to the 4 lsbs in the column address)
            buf[i * OLED_PAGE_SIZE + 6] = 0x40;             // Control byte: specify that the remainder of the transmission will be pixel data
        }
    }

    /// Return a mutable reference to the display buffer
    fn get_buffer(&self) -> &'static mut [u8] {
        unsafe { &mut OLED_BUFFER }
    }

}
