use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Scalar<T> {
    saved: T,
    curr: T,
    min: T,
    max: T,
    step: T,
}
impl<T: PartialOrd + Copy + Default + std::ops::Add<Output = T> + std::ops::Sub<Output = T>>
    Scalar<T>
{
    pub fn new(value: T, min: T, max: T, step: T) -> Self {
        Self {
            saved: value,
            curr: value,
            min,
            max,
            step,
        }
    }
    pub fn save(&mut self) {
        self.saved = self.curr;
    }
    pub fn saved(&self) -> T {
        self.saved
    }
    pub fn set_saved(&mut self, value: T) {
        self.saved = value;
    }

    pub fn curr(&self) -> T {
        self.curr
    }
    pub fn set_curr(&mut self, value: T) {
        self.curr = value;
    }

    pub fn step(&self) -> T {
        self.step
    }

    pub fn incr(&mut self) {
        let new_value = self.curr + self.step;
        if new_value <= self.max {
            self.curr = new_value;
        }
    }
    pub fn decr(&mut self) {
        let new_value = self.curr - self.step;
        if new_value >= self.min {
            self.curr = new_value;
        }
    }
    pub fn min(&self) -> T {
        self.min
    }
    pub fn max(&self) -> T {
        self.max
    }
}
