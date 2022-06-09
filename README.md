# Fluid
#### _A resource-constrained particle-based fluid simulation in Rust_

The goal of this project is to run a coarse, two dimensional, particle-based fluid simulation on an STM32F030K6 with only 4kb of RAM, and render the action to a small 128x64 monochrome OLED display at 30fps. 

## The simulation

This is a Lagrangian fluid simulation based on the paper by Simon Clavet, Phillipe Beaudoin, and Pierre Poulin: Particle-based Viscoelastic Fluid Simulation (2005). The basic algorithm is as follows:
* Update the velocity of each particle based on gravitational forces -- O(N)
* Update the velocity of each particle based on fluid viscosity -- O(N<sup>2</sup>)
* Record the previous position and predict the next based on particle velocity -- O(N)
* Update the position of each particle based on pressure impulses driven by particle density -- O(N<sup>2</sup>)
* Resolve collisions -- O(N)
* Revise the velocity of each particle to match the difference between the current and previous positions -- O(N)
 
 ## The display
 
 The display is a 128x64 pixel monochromatic OLED display driven by an SSD1306 controller. The display hardware is configured for I2C communication. Due to the monochromatic nature of the display, 8 pixels can be addressed by a single byte, which means that each frame can be represented in 1024 bytes. Including the overhead of I2C communication and inter-device communication, each frame transmission ultimately consists of 1080 bytes. With a goal of 30 frames per second, a minimum transmission frequency of roughly 300KHz is required, which is readily satisfied by the 400KHz fast-mode. Transmissions are handled via DMA to allow the processor to focus on simulating and drawing the fluid.
 
 ## The microcontroller
 
 An STM32F030K6 is the ultimate target for this project. With only 32kb of Flash, 4kb of RAM, a maximum clock of 48MHz, and no FPU, there's a little extra work necessary to make sure that resources are used wisely. 
 
 Current testing utilizes an STM32F051R8 because it is available on the STM32F0Discovery board that I had lying around. This is a functionally similar MCU with 64kb Flash and 8kb RAM.
 
 ## The software

Because this is a minimal bare metal environment, there is no heap and therefore we use Rust's nostd flag, so that only core platform-agnostic functionality is included. The crate is broken down into a few component modules:

* ##### DMA I2C interface  
I2C transmissions are handled via DMA. This interface consumes the I2C1 and DMA1 peripherals.

* ##### OLED driver  
A driver for the OLED that utilizes the DMA I2C interface to communicate with the SSD1306 controller. This provides pixel control to the rest of the system.

* ##### Fluid simulation  
A coarse, two-dimensional, particle-based fluid simulation. Currently operating 50 particles strong. Two major optimizations were necessary to get this working in real time on such a limited device:  fixed point arithmetic and roughly estimating vector magnitudes to avoid square root calculations.