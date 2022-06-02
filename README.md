# A resource-constrained particle-based fluid simulation

*This is currently a work in progress.*

The goal of this project is to run a coarse fluid simulation on an STM32F030K6U6 with only 4kb of RAM, and render the action to a small monochrome OLED display at 30fps. 

The software will be written in Rust. 

This will be a Lagrangian fluid simulation based on the paper by Simon Clavet, Phillipe Beaudoin, and Pierre Poulin: Particle-based Viscoelastic Fluid Simulation (2005)
