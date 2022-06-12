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
        const DT: FixedPt = FixedPt{ value: (0.9 * (1 << FixedPt::BASE) as f32) as i32 };

        // apply gravity to each particle
        self.apply_gravity(DT);

        // apply viscosity
        self.apply_viscosity(DT);

        // update positions based on current velocity
        self.apply_velocity(DT);

        // double density relaxation
        self.double_density_relaxation(DT);

        // resolve collisions
        self.resolve_collisions();

        // revise velocity based on final positions
        self.revise_velocity(DT);
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
            self.particles[i].density = FixedPtNearFar::ZERO;
            // compute density and near density
            for j in 0..self.particle_count() {
                if i == j { 
                    continue;
                }
                let distance = self.particles[i].distance_to(&self.particles[j]);
                if distance < self.particle_interaction_radius {
                    let linear_kernel = (self.particle_interaction_radius - distance) / self.particle_interaction_radius;
                    let quadratic_kernel = linear_kernel * linear_kernel;
                    let cubic_kernel = quadratic_kernel * linear_kernel;
                    let density_contibution = FixedPtNearFar {  
                        near: cubic_kernel,
                        far: quadratic_kernel,
                    };
                    self.particles[i].density += density_contibution;
                }
            }
            // compute pressure and near pressure
            self.particles[i].pressure.far = self.stiffness.far * (self.particles[i].density.far - self.target_density);
            self.particles[i].pressure.near = self.stiffness.near * self.particles[i].density.near;
            // apply pressure impulse between neighboring particles
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
                    let quadratic_kernel = linear_kernel * linear_kernel;
                    let pressure_impulse = direction * (pfar * linear_kernel + pnear * quadratic_kernel) * dt * dt;
                    self.particles[i].position -= pressure_impulse / 2;
                    self.particles[j].position += pressure_impulse / 2;
                }
            }
        }
    }

    fn resolve_collisions(&mut self) {
        for particle in self.particles.iter_mut() {
            // Ensure particles stay within defined boundaries
            particle.position.x = match particle.position.x {
                x if x < FixedPt::ZERO => FixedPt::ZERO,
                x if x > self.x_max => self.x_max,
                x => x,
            };
            particle.position.y = match particle.position.y {
                y if y < FixedPt::ZERO => FixedPt::ZERO,
                y if y > self.y_max => self.y_max,
                y => y,
            };
        }
    }

    fn revise_velocity(&mut self, dt: FixedPt) {
        for particle in self.particles.iter_mut() {
            particle.velocity = (particle.position - particle.previous_position) / dt;
        }
    }

    const PARTICLE_POSITIONS_INIT: [(i8, i8); 86] = [
        // F
        ( 0, 17),
        ( 0, 23),
        ( 0, 29),
        ( 0, 35),
        ( 6, 23),
        ( 6, 35),
        (12, 35),
        // L
        (21, 17),
        (21, 23),
        (21, 29),
        (21, 35),
        (27, 17),
        (33, 17),
        // U
        (42, 17),
        (42, 23),
        (42, 29),
        (42, 35),
        (48, 17),
        (54, 17),
        (54, 23),
        (54, 29),
        (54, 35),
        // I
        (63, 17),
        (63, 35),
        (69, 17),
        (69, 23),
        (69, 29),
        (69, 35),
        (75, 17),
        (75, 35),
        // D
        (84, 17),
        (84, 23),
        (84, 29),
        (84, 35),
        (90, 17),
        (90, 35),
        (96, 23),
        (96, 29),
        // [Drop]
	    (105, 14),
	    (105, 20),
	    (108,  8),
	    (108, 26),
	    (111, 32),
	    (114,  5),
	    (114, 38),
	    (117, 32),
	    (120,  8),
	    (120, 26),
	    (123, 14),
	    (123, 20),
        // overflow rows
	    (  3, 5),
	    ( 13, 5),
	    ( 23, 5),
	    ( 33, 5),
	    ( 43, 5),
	    ( 53, 5),
	    ( 63, 5),
	    ( 73, 5),
	    ( 83, 5),
	    ( 93, 5),
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