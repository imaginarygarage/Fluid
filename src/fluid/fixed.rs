/// Fractional precision for particle calculations without floating point math
/// (the stm32f030 does not hae an FPU)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedPt {
    pub value: i32,
}

impl FixedPt {
    pub const BASE: u8 = 16; // must be even
    pub const HALF_BASE: u8 = Self::BASE / 2;
    pub const ZERO: FixedPt = FixedPt::from_i8(0);

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
pub struct FixedPtVec2D {
    pub x: FixedPt,
    pub y: FixedPt
}

impl FixedPtVec2D {
    // Floating point arithmetic is not supported in const fn, set value manually
    const SQRT_2_MINUS_1: FixedPt = FixedPt{ value: (0.41421356 * (1 << FixedPt::BASE) as f64) as i32 };
    //const SQRT_2_MINUS_1: FixedPt = FixedPt::from_f32(0.41421356);
    pub const ZERO: FixedPtVec2D = FixedPtVec2D { x: FixedPt::from_i8(0), y: FixedPt::from_i8(0) };
    pub const ORIGIN: FixedPtVec2D = FixedPtVec2D::ZERO;

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
pub struct FixedPtNearFar {
    pub near: FixedPt,
    pub far: FixedPt
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
pub struct FixedPtViscosity {
    pub sigma: FixedPt,
    pub beta: FixedPt
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