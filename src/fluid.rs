#![no_std]
#![no_main]

//use fixed::types::I16F16;

// mod oled;
// use oled::OLEDDriver;

/// Fractional precision for particle calculations without floating point math
/// (the stm32f030 does not hae an FPU)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FixedPt {
    value: i32,
}

impl FixedPt {
    const BASE: u8 = 16; // must be even
    const HALF_BASE: u8 = Self::BASE / 2;
    const ZERO: FixedPt = FixedPt::from_i8(0);

    // TODO: make const when floating point arithmetic is supported in const fn
    pub fn from_f32(value: f32) -> FixedPt {
        FixedPt { 
            value: (value * (1 << Self::BASE) as f32) as i32,
        }
    }

    pub const fn from_i8(value: i8) -> FixedPt {
        FixedPt { 
            value: (value as i32) << Self::BASE,
        }
    }

    pub const fn abs(self) -> FixedPt {
        FixedPt {
            value: {
                if self.value < 0 {
                    -self.value
                }
                else {
                    self.value
                }
            }
        }
    }

    pub const fn to_i8(&self) -> i8 {
        (self.value >> Self::BASE) as i8
    }
}

impl core::ops::Add for FixedPt {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        FixedPt { 
            value: self.value + rhs.value
        }
    }
}

impl core::ops::AddAssign for FixedPt {
    fn add_assign(&mut self, rhs: FixedPt) {
        self.value += rhs.value;
    }
}

impl core::ops::Sub for FixedPt {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        FixedPt { 
            value: self.value - rhs.value
        }
    }
}

impl core::ops::SubAssign for FixedPt {
    fn sub_assign(&mut self, rhs: Self) {
        self.value -= rhs.value;
    }
}

impl core::ops::Mul for FixedPt {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        FixedPt { 
            value: (self.value >> Self::HALF_BASE) * (rhs.value >> Self::HALF_BASE)
        }
    }
}

impl core::ops::Div for FixedPt {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        FixedPt { 
            value: (self.value << Self::HALF_BASE) / (rhs.value << Self::HALF_BASE)
        }
    }
}

impl core::ops::Div<i32> for FixedPt {
    type Output = Self;
    fn div(self, rhs: i32) -> Self {
        FixedPt { 
            value: self.value / rhs,
        }
    }
}

#[derive(Copy, Clone)]
struct FixedPtVec2D {
    x: FixedPt,
    y: FixedPt
}

impl FixedPtVec2D {
    // Floating point arithmetic is not supported in const fn, set value manually
    const SQRT_2_MINUS_1: FixedPt = FixedPt{ value: (0.41421356 * (1 << FixedPt::BASE) as f64) as i32 };
    //const SQRT_2_MINUS_1: FixedPt = FixedPt::from_f32(0.41421356);
    const ZERO: FixedPtVec2D = FixedPtVec2D { x: FixedPt::from_i8(0), y: FixedPt::from_i8(0) };
    const ORIGIN: FixedPtVec2D = FixedPtVec2D::ZERO;

    pub fn from_i8s(x: i8, y: i8) -> Self {
        Self { 
            x: FixedPt::from_i8(x), 
            y: FixedPt::from_i8(y)
        }
    }

    pub fn dot(&self, vector_2: &Self) -> FixedPt {
        self.x * vector_2.x + self.y * vector_2.y
    }

    pub fn distance_to(&self, position: &Self) -> FixedPt {
        self.vector_to(position).magnitude()
    }

    pub fn magnitude(&self) -> FixedPt {
        let dx = self.x.abs();
        let dy = self.y.abs();
        let a = core::cmp::max(dx, dy);
        let b = core::cmp::min(dx, dy);
        a + b * Self::SQRT_2_MINUS_1
    }

    pub fn vector_to(&self, vector_2: &Self) -> Self {
        Self { 
            x: vector_2.x - self.x, 
            y: vector_2.y - self.y, 
        }
    }

    pub fn as_unit_vector(&self) -> Self {
        let magnitude = Self::ORIGIN.distance_to(self);
        Self { 
            x: self.x / magnitude, 
            y: self.y / magnitude, 
        }
    }
}

impl core::ops::Add for FixedPtVec2D {
    type Output = Self;
    fn add(self, rhs: FixedPtVec2D) -> Self {
        FixedPtVec2D { 
            x: self.x + rhs.x, 
            y: self.y + rhs.y 
        }
    }
}

impl core::ops::AddAssign for FixedPtVec2D {
    fn add_assign(&mut self, rhs: FixedPtVec2D) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl core::ops::Sub for FixedPtVec2D {
    type Output = Self;
    fn sub(self, rhs: FixedPtVec2D) -> Self {
        FixedPtVec2D { 
            x: self.x - rhs.x, 
            y: self.y - rhs.y 
        }
    }
}

impl core::ops::SubAssign for FixedPtVec2D {
    fn sub_assign(&mut self, rhs: FixedPtVec2D) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl core::ops::Mul<FixedPt> for FixedPtVec2D {
    type Output = Self;
    fn mul(self, rhs: FixedPt) -> Self {
        FixedPtVec2D { 
            x: self.x * rhs, 
            y: self.y * rhs 
        }
    }
}

impl core::ops::Div<FixedPt> for FixedPtVec2D {
    type Output = Self;
    fn div(self, rhs: FixedPt) -> Self {
        FixedPtVec2D { 
            x: self.x / rhs, 
            y: self.y / rhs 
        }
    }
}

impl core::ops::Div<i32> for FixedPtVec2D {
    type Output = Self;
    fn div(self, rhs: i32) -> Self {
        FixedPtVec2D { 
            x: self.x / rhs, 
            y: self.y / rhs 
        }
    }
}

#[derive(Copy, Clone)]
struct FixedPtNearFar {
    near: FixedPt,
    far: FixedPt
}

impl FixedPtNearFar {
    pub fn from_i8s(near: i8, far: i8) -> Self {
        Self { 
            near: FixedPt::from_i8(near), 
            far: FixedPt::from_i8(far)
        }
    }
    pub fn from_f32s(near: f32, far: f32) -> Self {
        Self { 
            near: FixedPt::from_f32(near), 
            far: FixedPt::from_f32(far)
        }
    }
}

#[derive(Copy, Clone)]
struct FixedPtViscosity {
    sigma: FixedPt,
    beta: FixedPt
}

impl FixedPtViscosity {
    pub fn from_i8s(sigma: i8, beta: i8) -> Self {
        Self { 
            sigma: FixedPt::from_i8(sigma), 
            beta: FixedPt::from_i8(beta)
        }
    }
    pub fn from_f32s(sigma: f32, beta: f32) -> Self {
        Self { 
            sigma: FixedPt::from_f32(sigma), 
            beta: FixedPt::from_f32(beta)
        }
    }
}

#[derive(Copy, Clone)]
pub struct Particle {
    position: FixedPtVec2D,
    previous_position: FixedPtVec2D,
    velocity: FixedPtVec2D,
    pressure: FixedPtNearFar,
    density: FixedPtNearFar,
}

impl Particle {
    pub fn new(x: i8, y: i8) -> Self {
        Self {
            position: FixedPtVec2D::from_i8s(x, y),
            previous_position: FixedPtVec2D::from_i8s(x, y),
            velocity: FixedPtVec2D::from_i8s(0, 0),
            pressure: FixedPtNearFar::from_i8s(0, 0),
            density: FixedPtNearFar::from_i8s(0, 0),
        }
    }

    pub fn distance_to(&self, particle: &Self) -> FixedPt {
        self.position.distance_to(&particle.position)
    }

    pub fn vector_to(&self, particle: &Self) -> FixedPtVec2D {
        self.position.vector_to(&particle.position)
    }

    pub fn get_display_position(&self) -> (i8, i8) {
        (self.position.x.to_i8(), self.position.y.to_i8())
    }

    pub fn set_position(&mut self, x: i8, y: i8) {
        self.position = FixedPtVec2D::from_i8s(x, y);
    }
}

pub struct Fluid<const N: usize> {
    particles: [Particle; N],
    particle_interaction_radius: FixedPt,
    stiffness: FixedPtNearFar,
    target_density: FixedPt,
    viscosity: FixedPtViscosity, 
    gravity: FixedPtVec2D,
    x_max: FixedPt,
    y_max: FixedPt,
}

impl<const N: usize> Fluid<N> {
    pub fn new(width: i8, height: i8) -> Self {
        // Create the fluid struct
        let mut fluid = Fluid {
            particles: [Particle::new(0, 0); N],
            particle_interaction_radius: FixedPt::from_f32(16.0),
            stiffness: FixedPtNearFar::from_f32s(3.0, 2.0),
            target_density: FixedPt::from_f32(3.5),
            viscosity: FixedPtViscosity::from_f32s(0.0, 0.05),
            gravity: FixedPtVec2D::from_i8s(0, 0),
            x_max: FixedPt::from_i8(width - 1),
            y_max: FixedPt::from_i8(height - 1),
        };

        // Initialize Particle Positions
        for (i, particle) in fluid.particles.iter_mut().enumerate() {
            if i < Self::PARTICLE_POSITIONS_INIT.len() {
                let (x, y) = Self::PARTICLE_POSITIONS_INIT[i];
                particle.set_position(x, y);
            }
        }

        fluid
    }

    pub fn step(&mut self) {
        //todo: do something better with this timestep
        const dt: FixedPt = FixedPt{ value: (0.9 * (1 << FixedPt::BASE) as f32) as i32 };

        // apply gravity to each particle
        self.apply_gravity(dt);

        // apply viscosity
        self.apply_viscosity(dt);

        // update positions based on current velocity
        self.apply_velocity(dt);

        // double density relaxation
        self.double_density_relaxation(dt);

        // resolve collisions
        self.resolve_collisions();

        // revise velocity based on final positions
        self.revise_velocity(dt);
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    pub fn get_particles(&self) -> &[Particle] {
        &self.particles
    }

    fn apply_gravity(&mut self, dt: FixedPt) {
        let delta_v = self.gravity * dt;
        for particle in &mut self.particles {
            particle.velocity += delta_v;
        }
    }

    fn apply_viscosity(&mut self, dt: FixedPt) {
        for i in 0..self.particles.len() {
            for j in (i + 1)..self.particles.len() {
                let distance_vector = self.particles[i].vector_to(&self.particles[j]);
                let distance = distance_vector.magnitude();
                if distance < self.particle_interaction_radius {
                    // get the unit vector pointing from this particle to the neighbor
                    let direction = distance_vector / distance;
                    let relative_velocity = self.particles[j].vector_to(&self.particles[i]);
                    // calculate the inward radial velocity
                    let irv = relative_velocity.dot(&direction);
                    if irv > FixedPt::ZERO {
                        // apply linear and quadratic viscosity impulses
                        let q = distance / self.particle_interaction_radius;
                        let viscosity_impulse = direction * dt * (FixedPt::from_i8(1) - q) * (self.viscosity.sigma * irv + self.viscosity.beta * irv * irv);
                        self.particles[i].velocity -= viscosity_impulse / 2;
                        self.particles[j].velocity += viscosity_impulse / 2;
                    }
                    
                }
            }
        }
    }

    fn apply_velocity(&mut self, dt: FixedPt) {
        for particle in self.particles.iter_mut() {
            particle.previous_position = particle.position;
            particle.position += particle.velocity * dt;
        }
    }

    fn double_density_relaxation(&mut self, dt: FixedPt) {
        for i in 0..self.particle_count() {
            // reset density
            self.particles[i].density.near = FixedPt::ZERO;
            self.particles[i].density.far = FixedPt::ZERO;
            // compute density and near density
            for j in 0..self.particle_count() {
                if i == j { 
                    continue;
                }
                let distance = self.particles[i].distance_to(&self.particles[j]);
                if distance < self.particle_interaction_radius {
                    let one_minus_q = (self.particle_interaction_radius - distance) / self.particle_interaction_radius;
                    self.particles[i].density.far += one_minus_q * one_minus_q;
                    self.particles[i].density.near += one_minus_q * one_minus_q * one_minus_q;
                }
            }
            // compute pressure and near pressure
            self.particles[i].pressure.far = self.stiffness.far * (self.particles[i].density.far - self.target_density);
            self.particles[i].pressure.near = self.stiffness.near * self.particles[i].density.near;
            // apply displacement
            let mut displacement = FixedPtVec2D::ZERO;
            for j in 0..self.particle_count() {
                if i == j { 
                    continue;
                }
                let distance_vector = self.particles[i].vector_to(&self.particles[j]);
                let distance = distance_vector.magnitude();
                if distance < self.particle_interaction_radius {
                    let one_minus_q = (self.particle_interaction_radius - distance) / self.particle_interaction_radius;
                    let direction = distance_vector / distance;
                    let pnear = self.particles[i].pressure.near;
                    let pfar = self.particles[i].pressure.far;
                    let pressure_impulse = direction * (pfar * one_minus_q + pnear * one_minus_q * one_minus_q) * dt * dt;
                    self.particles[j].position += pressure_impulse / 2;
                    displacement -= pressure_impulse / 2;
                }
            }
            self.particles[i].position += displacement;
        }
    }

    fn resolve_collisions(&mut self) {
        for particle in self.particles.iter_mut() {
            if particle.position.x < FixedPt::ZERO {
                particle.position.x = FixedPt::ZERO;
            }
            else if particle.position.x > self.x_max {
                particle.position.x = self.x_max;
            }
            if particle.position.y < FixedPt::ZERO {
                particle.position.y = FixedPt::ZERO;
            }
            else if particle.position.y > self.y_max {
                particle.position.y = self.y_max;
            }
        }
    }

    fn revise_velocity(&mut self, dt: FixedPt) {
        for particle in self.particles.iter_mut() {
            particle.velocity = (particle.position - particle.previous_position) / dt;
        }
    }

    const PARTICLE_POSITIONS_INIT: [(i8, i8); 86] = [
        // F
        ( 5, 11),
        ( 5, 17),
        ( 5, 23),
        ( 5, 29),
        (11, 17),
        (11, 29),
        (17, 29),
        // L
        (23, 11),
        (23, 17),
        (23, 23),
        (23, 29),
        (29, 11),
        (35, 11),
        // U
        (41, 11),
        (41, 17),
        (41, 23),
        (41, 29),
        (47, 11),
        (53, 11),
        (53, 17),
        (53, 23),
        (53, 29),
        // I
        (59, 11),
        (59, 29),
        (65, 11),
        (65, 17),
        (65, 23),
        (65, 29),
        (71, 11),
        (71, 29),
        // D
        (77, 11),
        (77, 17),
        (77, 23),
        (77, 29),
        (83, 11),
        (83, 29),
        (89, 17),
        (89, 23),
        // [Drop]
	    ( 95, 14),
	    ( 95, 20),
	    ( 98,  8),
	    ( 98, 26),
	    (101, 32),
	    (104,  5),
	    (104, 38),
	    (107, 32),
	    (110,  8),
	    (110, 26),
	    (113, 14),
	    (113, 20),
        // overflow rows
	    (29, 44),
	    (35, 44),
	    (41, 44),
	    (47, 44),
	    (53, 44),
	    (59, 44),
	    (65, 44),
	    (71, 44),
	    (77, 44),
	    (83, 44),
	    (89, 44),
	    (95, 44),
	    (29, 50),
	    (35, 50),
	    (41, 50),
	    (47, 50),
	    (53, 50),
	    (59, 50),
	    (65, 50),
	    (71, 50),
	    (77, 50),
	    (83, 50),
	    (89, 50),
	    (95, 50),
	    (29, 56),
	    (35, 56),
	    (41, 56),
	    (47, 56),
	    (53, 56),
	    (59, 56),
	    (65, 56),
	    (71, 56),
	    (77, 56),
	    (83, 56),
	    (89, 56),
	    (95, 56),
    ];
}