use stm32f0xx_hal::pac::{I2C1, DMA1};
use stm32f0xx_hal::rcc::Rcc;

mod dmai2c;
use dmai2c::{DMAi2c, I2CBuffer};


// The OLED display used here is a 128 pixel wide by 64 pixel
// high monochrome display with an SSD1306 controller.
const OLED_PXLS_X: usize = 128;
const OLED_PXLS_Y: usize = 64;
const OLED_COLS: usize = OLED_PXLS_X;

// An OLED page represents a row of the display 8 pixels tall.
// Each column in this row is represented by a u8 value, where
// a 1 in the LSB represents the top pixel in the on state.
const OLED_PAGES: usize = OLED_PXLS_Y / 8;
const OLED_PAGE_HEADER_SIZE: usize = 7;
const OLED_PAGE_SIZE: usize = OLED_COLS + OLED_PAGE_HEADER_SIZE;


// A list of commands for initializing the OLED display.
// TODO: This really shouldn't need to be mutable. 
static mut OLED_INIT_CMDS: [I2CBuffer; 18] = [
    I2CBuffer{ data: &mut [0, 0xAE],             tx_size: 5_u8 },   //Put OLED display into sleep mode
    I2CBuffer{ data: &mut [0, 0x81, 120],        tx_size: 5_u8 },   //Set Contrast Value (Command: 0x81) to an 8-bit value ( < 256 )
    I2CBuffer{ data: &mut [0, 0xA4],             tx_size: 5_u8 },   //Display RAM content (0xA5: all pixels on, regardless of RAM contents)
    I2CBuffer{ data: &mut [0, 0xA6],             tx_size: 5_u8 },   //Set display ram so that 1 = on (0xA7: inverted)
    I2CBuffer{ data: &mut [0, 0x20, 0x00],       tx_size: 5_u8 },   //Set Memory Addressing Mode to Horizontal Addressing Mode(00): Column address pointer is incremented by one unless end address has been reached, at which point column address is reset to 0 and page address is incremented by one. Page is reset if already at end address.
    I2CBuffer{ data: &mut [0, 0x21, 0x00, 127],  tx_size: 5_u8 },   //Set Column start address to 0 and column end address to 127
    I2CBuffer{ data: &mut [0, 0x22, 0x00, 0x07], tx_size: 5_u8 },   //Set Page start address to 0 and page end address to 7
    I2CBuffer{ data: &mut [0, 0x40],             tx_size: 5_u8 },   //Set display start line to 0
    I2CBuffer{ data: &mut [0, 0xA1],             tx_size: 5_u8 },   //Map column address 0 to SEGMENT127 - this way column address zero is on the left side of the screen in my desired orientation
    I2CBuffer{ data: &mut [0, 0xA8, 63],         tx_size: 5_u8 },   //Set MUX ratio to n + 1. Valid n values: 15-63
    I2CBuffer{ data: &mut [0, 0xC0],             tx_size: 5_u8 },   //Scan from COM0 to COM[n-1]; n = mux ratio (C8: inverted)
    I2CBuffer{ data: &mut [0, 0xD3, 0],          tx_size: 5_u8 },   //Set display offset vertical shift to 0
    I2CBuffer{ data: &mut [0, 0xDA, 0b00010010], tx_size: 5_u8 },   //Set alternative COM pin configuration and disable COM left/right remap
    I2CBuffer{ data: &mut [0, 0xD5, 0xC0],       tx_size: 5_u8 },   //Set Display Clock Oscillator speed to 12/16, prescaled by 0.
    I2CBuffer{ data: &mut [0, 0xD9, 0x22],       tx_size: 5_u8 },   //Set pre-charge period to 2 clocks for phase 1, and 2 clocks for phase 2
    I2CBuffer{ data: &mut [0, 0xDB, 0x20],       tx_size: 5_u8 },   //Set Vcomh deselect level 0x20 ~ 0.77Vcc (0x00: 0.65Vcc; 0x30: 0.83Vcc)*/
    I2CBuffer{ data: &mut [0, 0x8D, 0x14],       tx_size: 5_u8 },   //Enable Charge Pump when display is on
    I2CBuffer{ data: &mut [0, 0xAF],             tx_size: 5_u8 },   //Turn on OLED Display
];

// OLED buffers
static mut OLED_BUFFER_1: I2CBuffer = I2CBuffer{ data:&mut [0; OLED_PAGE_SIZE * OLED_PAGES], tx_size: OLED_PAGE_SIZE as u8 };
static mut OLED_BUFFER_2: I2CBuffer = I2CBuffer{ data:&mut [0; OLED_PAGE_SIZE * OLED_PAGES], tx_size: OLED_PAGE_SIZE as u8 };


enum OLEDBufferID {
    Buffer1,
    Buffer2
}


pub struct OLEDDriver {
    draw_buffer: OLEDBufferID,
    disp_buffer: OLEDBufferID
}

impl OLEDDriver {
    /// Create and initialize a new OLED buffer
    pub fn new(i2c: I2C1, dma: DMA1, rcc: &mut Rcc) -> OLEDDriver {
        // Initialize the DMA I2C peripheral
        DMAi2c::init(i2c, dma, rcc);

        //TODO: wait 100ms for OLED to power up

        // initialize the OLED
        unsafe {
            for cmd in &OLED_INIT_CMDS {
                DMAi2c::tx(cmd);
            }
        }

        // Initialize buffers and create the driver struct
        unsafe {
            OLEDDriver::init_oled_buffer(&mut OLED_BUFFER_1);
            OLEDDriver::init_oled_buffer(&mut OLED_BUFFER_2);
        }

        // Return the OLED driver
        OLEDDriver {
            draw_buffer: OLEDBufferID::Buffer1,
            disp_buffer: OLEDBufferID::Buffer2,
        }
    }

    /// Turn off every pixel
    pub fn clear(&self) {
        unsafe {
            let buffer = self.get_draw_buffer();
            for i in 0..buffer.data.len() {
                if i % OLED_PAGE_SIZE >= OLED_PAGE_HEADER_SIZE {
                    buffer.data[i] = 0;
                }
            }
            // for (i, byte) in buffer.data.iter_mut().enumerate() {
            //     if i % OLED_PAGE_SIZE > OLED_PAGE_HEADER_SIZE {
            //         *byte = 0;
            //     }
            // }
        }
    }

    /// Invert the OLED buffer
    pub fn invert(&mut self) {
        unsafe {
            let buffer = self.get_draw_buffer();
            for i in 0..buffer.data.len() {
                if i % OLED_PAGE_SIZE > OLED_PAGE_HEADER_SIZE {
                    buffer.data[i] = !buffer.data[i];
                }
            }
            // for (i, byte) in buffer.data.iter_mut().enumerate() {
            //     if i % OLED_PAGE_SIZE < OLED_PAGE_HEADER_SIZE {
            //         *byte = !(*byte);
            //     }
            // }
        }
    }

    /// Set a given pixel to be on or off
    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        unsafe {
            let buffer = self.get_draw_buffer();
            let row = y / 8;
            let bit = y % 8;
            let idx = row * OLED_PAGE_SIZE + OLED_PAGE_HEADER_SIZE + x;
            if on {
                buffer.data[idx] |= 1 << bit;
            }
            else {
                buffer.data[idx] &= !(1 << bit);
            }
            // let byte = &mut buffer.data[row * OLED_PAGE_SIZE + OLED_PAGE_HEADER_SIZE + x];
            // if on {
            //     *byte |= 1 << bit;
            // }
            // else {
            //     *byte &= !(1 << bit);
            // }
        }
    }

    /// Transmit the current draw buffer to the OLED.
    /// This also swaps the buffers and clears the new draw buffer.
    pub fn tx_frame(&mut self) {
        self.swap_buffers();
        unsafe {
            DMAi2c::tx(self.get_display_buffer());
        }
        self.clear();
    }

    /// Initialize an existing OLED buffer
    fn init_oled_buffer(buf: &'static mut I2CBuffer) {
        // set up buffer page headers
        for i in 0..OLED_PAGES {
            // TODO: document constants (OLED commands)
            buf.data[i * OLED_PAGE_SIZE + 0] = 0x80;
            buf.data[i * OLED_PAGE_SIZE + 1] = 0xB0 + i as u8;
            buf.data[i * OLED_PAGE_SIZE + 2] = 0x80;
            buf.data[i * OLED_PAGE_SIZE + 3] = 0b00010000;
            buf.data[i * OLED_PAGE_SIZE + 4] = 0x80;
            buf.data[i * OLED_PAGE_SIZE + 5] = 0x00;
            buf.data[i * OLED_PAGE_SIZE + 6] = 0x40;
        }
    }

    /// Return a mutable reference to the current display buffer
    unsafe fn get_display_buffer(&self) -> &'static mut I2CBuffer {
        match self.disp_buffer {
            OLEDBufferID::Buffer1 => &mut OLED_BUFFER_1,
            OLEDBufferID::Buffer2 => &mut OLED_BUFFER_2
        }
    }

    /// Return a mutable reference to the current draw buffer
    unsafe fn get_draw_buffer(&self) -> &'static mut I2CBuffer {
        match self.draw_buffer {
            OLEDBufferID::Buffer1 => &mut OLED_BUFFER_1,
            OLEDBufferID::Buffer2 => &mut OLED_BUFFER_2
        }
    }

    /// Swap the display and draw buffers
    fn swap_buffers(&mut self) {
        match self.disp_buffer {
            OLEDBufferID::Buffer1 => {
                self.disp_buffer = OLEDBufferID::Buffer2;
                self.draw_buffer = OLEDBufferID::Buffer1;
            }
            OLEDBufferID::Buffer2 => {
                self.disp_buffer = OLEDBufferID::Buffer1;
                self.draw_buffer = OLEDBufferID::Buffer2;
            }
        }
    }

}
