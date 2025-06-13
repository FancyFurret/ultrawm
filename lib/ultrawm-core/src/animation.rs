use std::collections::VecDeque;
use std::time::Instant;

pub trait Animatable {
    fn animate_frame(&mut self) -> bool;
}

/// Trait for types that can be interpolated.
pub trait Interpolatable: Sized + Clone {
    fn interpolate(&self, target: &Self, t: f64) -> Self;
}

impl Interpolatable for f32 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t as f32
    }
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

pub struct Animator<T>
where
    T: Interpolatable,
{
    pub from: T,
    pub to: T,
    pub duration: u32,
    pub start_time: Option<Instant>,
    pub ease_fn: fn(f64) -> f64,
    pub animating: bool,
    pub last_value: T,
    frame_times: VecDeque<Instant>,
    last_frame_time: Option<Instant>,
}

impl<T> Animator<T>
where
    T: Interpolatable,
{
    pub fn new(from: T, to: T, ease_fn: fn(f64) -> f64) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
            duration: 0,
            start_time: None,
            ease_fn,
            animating: false,
            last_value: from,
            frame_times: VecDeque::with_capacity(60), // Store last 60 frames for FPS calculation
            last_frame_time: None,
        }
    }

    pub fn start_from(&mut self, from: T, to: T, duration: u32) {
        let now = Instant::now();
        self.from = from.clone();
        self.to = to.clone();
        self.duration = duration;
        self.start_time = Some(now);
        self.animating = true;
        self.last_value = from;
        self.frame_times.clear();
        self.last_frame_time = Some(now);
    }

    pub fn start(&mut self, to: T, duration: u32) {
        self.start_from(self.last_value.clone(), to, duration);
    }

    /// Returns Some(new_value) if animating, None if finished
    pub fn update(&mut self) -> Option<T> {
        let now = Instant::now();
        if !self.animating {
            return None;
        }

        // Track frame timing
        if let Some(_last_frame) = self.last_frame_time {
            self.frame_times.push_back(now);
            if self.frame_times.len() > 60 {
                self.frame_times.pop_front();
            }
        }
        self.last_frame_time = Some(now);

        if self.duration == 0 {
            self.animating = false;
            self.last_value = self.to.clone();
            return Some(self.to.clone());
        }

        let start = self.start_time.unwrap();
        let elapsed = (now - start).as_millis() as f64;
        let mut t = (elapsed / (self.duration as f64)).clamp(0.0, 1.0);
        if t >= 1.0 {
            t = 1.0;
            self.animating = false;
            self.print_fps();
        }
        let eased_t = (self.ease_fn)(t);
        let value = self.from.interpolate(&self.to, eased_t);
        self.last_value = value.clone();
        Some(value)
    }

    pub fn print_fps(&self) {
        if self.frame_times.len() < 2 {
            return;
        }

        let total_duration = *self.frame_times.back().unwrap() - *self.frame_times.front().unwrap();
        let fps = (self.frame_times.len() as f64 - 1.0) / total_duration.as_secs_f64();
        println!("Animation completed with average FPS: {:.1}", fps);
    }

    pub fn is_animating(&self) -> bool {
        self.animating
    }

    pub fn current_value(&self) -> &T {
        &self.last_value
    }
}
