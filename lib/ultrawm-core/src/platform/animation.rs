use std::time::Instant;

/// Trait for types that can be interpolated.
pub trait Interpolatable: Sized + Clone {
    fn interpolate(&self, target: &Self, t: f64) -> Self;
}

impl Interpolatable for f64 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for u8 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        ((*self as f64) + ((*target as f64) - (*self as f64)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    }
}

// Bounds is expected to be imported from crate::platform
use crate::platform::Bounds;
impl Interpolatable for Bounds {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        let x = self.position.x + ((target.position.x - self.position.x) as f64 * t).round() as i32;
        let y = self.position.y + ((target.position.y - self.position.y) as f64 * t).round() as i32;
        let w = self.size.width as f64
            + ((target.size.width as i32 - self.size.width as i32) as f64 * t).round() as f64;
        let h = self.size.height as f64
            + ((target.size.height as i32 - self.size.height as i32) as f64 * t).round() as f64;
        Bounds {
            position: crate::platform::Position { x, y },
            size: crate::platform::Size {
                width: w as u32,
                height: h as u32,
            },
        }
    }
}

pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - 4.0 * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
}

pub struct Animator<T, F>
where
    T: Interpolatable,
    F: Fn(f64) -> f64,
{
    pub from: T,
    pub to: T,
    pub duration: f64,
    pub start_time: Option<Instant>,
    pub ease_fn: F,
    pub animating: bool,
    pub last_value: T,
}

impl<T, F> Animator<T, F>
where
    T: Interpolatable,
    F: Fn(f64) -> f64,
{
    pub fn new(from: T, to: T, ease_fn: F) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
            duration: 0.0,
            start_time: None,
            ease_fn,
            animating: false,
            last_value: from,
        }
    }

    pub fn start(&mut self, from: T, to: T, duration: f64) {
        let now = Instant::now();
        self.from = from.clone();
        self.to = to.clone();
        self.duration = duration;
        self.start_time = Some(now);
        self.animating = true;
        self.last_value = from;
    }

    /// Returns Some(new_value) if animating, None if finished
    pub fn update(&mut self) -> Option<T> {
        let now = Instant::now();
        if !self.animating {
            return None;
        }

        if self.duration == 0.0 {
            self.animating = false;
            return Some(self.to.clone());
        }

        let start = self.start_time.unwrap();
        let elapsed = (now - start).as_secs_f64();
        let mut t = (elapsed / self.duration).clamp(0.0, 1.0);
        if t >= 1.0 {
            t = 1.0;
            self.animating = false;
        }
        let eased_t = (self.ease_fn)(t);
        let value = self.from.interpolate(&self.to, eased_t);
        self.last_value = value.clone();
        Some(value)
    }

    pub fn is_animating(&self) -> bool {
        self.animating
    }

    pub fn current_value(&self) -> &T {
        &self.last_value
    }
}
