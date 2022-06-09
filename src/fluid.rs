mod fixed;
use fixed::{FixedPt, FixedPtVec2D, FixedPtNearFar, FixedPtViscosity};


#[derive(Copy, Clone)]
pub struct Particle {
    position: FixedPtVec2D,
    previous_position: FixedPtVec2D,
    velocity: FixedPtVec2D,
    pub pressure: FixedPtNearFar,
    pub density: FixedPtNearFar,
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

    /// The speed at which the particles are approaching one another is the
    /// dot product of the difference in velocity between the two particles
    /// and unit vector pointing from this particle to the other.
    /// Otherwise referred to as the inward radial velocity.
    pub fn approach_speed_of(&self, particle: &Self) -> FixedPt {
        let direction = self.position.vector_to(&particle.position).unit();
        let velocity_diff = particle.velocity.vector_to(&self.velocity);
        velocity_diff.dot(&direction)
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
    pub target_density: FixedPt,
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
            stiffness: FixedPtNearFar::from_f32s(4.0, 1.5),
            target_density: FixedPt::from_f32(2.5),
            viscosity: FixedPtViscosity::from_f32s(0.0, 0.10),
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

    pub fn set_gravity(&mut self, gx: f32, gy: f32) {
        self.gravity = FixedPtVec2D::from_f32s(gx, gy);
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
        for i in 0..self.particle_count() {
            for j in (i + 1)..self.particle_count() {
                let distance_vector = self.particles[i].vector_to(&self.particles[j]);
                let distance = distance_vector.magnitude();
                if distance < self.particle_interaction_radius && distance > FixedPt::ZERO {
                    // get the unit vector pointing from this particle to the neighbor
                    let direction = distance_vector / distance;
                    // calculate the inward radial velocity
                    let irv = self.particles[i].approach_speed_of(&self.particles[j]);
                    if irv > FixedPt::ZERO {
                        // apply the linear viscosity kernel and quadratic viscosity impulses
                        let viscosity_kernel = FixedPt::from_i8(1) - distance / self.particle_interaction_radius;
                        let viscosity_impulse = direction * viscosity_kernel * (self.viscosity.sigma * irv + self.viscosity.beta * irv * irv) * dt;
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
                    let linear_kernel = (self.particle_interaction_radius - distance) / self.particle_interaction_radius;
                    let density_contibution = FixedPtNearFar { 
                        far: linear_kernel * linear_kernel, 
                        near: linear_kernel * linear_kernel * linear_kernel,
                    };
                    self.particles[i].density += density_contibution;
                }
            }
            // compute pressure and near pressure
            self.particles[i].pressure.far = self.stiffness.far * (self.particles[i].density.far - self.target_density);
            self.particles[i].pressure.near = self.stiffness.near * self.particles[i].density.near;
            // apply pressure displacement
            for j in 0..self.particle_count() {
                if i == j { 
                    continue;
                }
                let distance_vector = self.particles[i].vector_to(&self.particles[j]);
                let distance = distance_vector.magnitude();
                if distance < self.particle_interaction_radius && distance > FixedPt::ZERO {
                    let direction = distance_vector / distance;
                    let pnear = self.particles[i].pressure.near;
                    let pfar = self.particles[i].pressure.far;
                    let linear_kernel = (self.particle_interaction_radius - distance) / self.particle_interaction_radius;
                    let pressure_impulse = direction * (pfar * linear_kernel + pnear * linear_kernel * linear_kernel) * dt * dt;
                    self.particles[i].position -= pressure_impulse / 2;
                    self.particles[j].position += pressure_impulse / 2;
                }
            }
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