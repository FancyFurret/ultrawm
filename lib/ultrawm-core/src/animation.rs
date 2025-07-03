use log::trace;
use std::collections::VecDeque;
use std::fmt::Debug;
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
        // Convert everything to f64 first to avoid intermediate rounding
        let start_x = self.position.x as f64;
        let start_y = self.position.y as f64;
        let start_w = self.size.width as f64;
        let start_h = self.size.height as f64;

        let end_x = target.position.x as f64;
        let end_y = target.position.y as f64;
        let end_w = target.size.width as f64;
        let end_h = target.size.height as f64;

        // Do all calculations in f64
        let x = start_x + (end_x - start_x) * t;
        let y = start_y + (end_y - start_y) * t;
        let w = start_w + (end_w - start_w) * t;
        let h = start_h + (end_h - start_h) * t;

        // Round only at the very end when converting back to integers
        Bounds {
            position: crate::platform::Position {
                x: x.round() as i32,
                y: y.round() as i32,
            },
            size: crate::platform::Size {
                width: w.round() as u32,
                height: h.round() as u32,
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
    T: Interpolatable + Debug,
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
            // Can be uncommented for debugging
            // self.print_fps();
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
        trace!("Animation completed with average FPS: {fps:.1}");
    }

    pub fn is_animating(&self) -> bool {
        self.animating
    }

    pub fn current_value(&self) -> &T {
        &self.last_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Bounds;
    use std::time::Duration;

    // Helper function for linear easing (no easing)
    fn linear(t: f64) -> f64 {
        t
    }

    // === Interpolatable Tests ===

    #[test]
    fn test_f32_interpolate_midpoint() {
        let start = 0.0f32;
        let end = 10.0f32;
        let result = start.interpolate(&end, 0.5);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_f32_interpolate_start() {
        let start = 5.0f32;
        let end = 15.0f32;
        let result = start.interpolate(&end, 0.0);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_f32_interpolate_end() {
        let start = 5.0f32;
        let end = 15.0f32;
        let result = start.interpolate(&end, 1.0);
        assert!((result - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_f32_interpolate_quarter() {
        let start = 0.0f32;
        let end = 100.0f32;
        let result = start.interpolate(&end, 0.25);
        assert!((result - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_f64_interpolate() {
        let start = 0.0f64;
        let end = 10.0f64;
        let result = start.interpolate(&end, 0.5);
        assert!((result - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_u8_interpolate_midpoint() {
        let start = 0u8;
        let end = 255u8;
        let result = start.interpolate(&end, 0.5);
        assert_eq!(result, 128); // Should round to nearest
    }

    #[test]
    fn test_u8_interpolate_rounding() {
        let start = 0u8;
        let end = 10u8;
        let result = start.interpolate(&end, 0.33); // Should be 3.3, rounds to 3
        assert_eq!(result, 3);

        let result = start.interpolate(&end, 0.37); // Should be 3.7, rounds to 4
        assert_eq!(result, 4);
    }

    #[test]
    fn test_bounds_interpolate_position() {
        let start = Bounds::new(0, 0, 100, 100);
        let end = Bounds::new(100, 200, 100, 100);
        let result = start.interpolate(&end, 0.5);

        assert_eq!(result.position.x, 50);
        assert_eq!(result.position.y, 100);
        assert_eq!(result.size.width, 100);
        assert_eq!(result.size.height, 100);
    }

    #[test]
    fn test_bounds_interpolate_size() {
        let start = Bounds::new(0, 0, 100, 100);
        let end = Bounds::new(0, 0, 200, 300);
        let result = start.interpolate(&end, 0.5);

        assert_eq!(result.position.x, 0);
        assert_eq!(result.position.y, 0);
        assert_eq!(result.size.width, 150);
        assert_eq!(result.size.height, 200);
    }

    #[test]
    fn test_bounds_interpolate_full() {
        let start = Bounds::new(10, 20, 100, 150);
        let end = Bounds::new(50, 80, 300, 250);
        let result = start.interpolate(&end, 0.25);

        // Position: start + (end - start) * 0.25
        assert_eq!(result.position.x, 20); // 10 + (50-10)*0.25 = 10 + 10 = 20
        assert_eq!(result.position.y, 35); // 20 + (80-20)*0.25 = 20 + 15 = 35
                                           // Size: start + (end - start) * 0.25
        assert_eq!(result.size.width, 150); // 100 + (300-100)*0.25 = 100 + 50 = 150
        assert_eq!(result.size.height, 175); // 150 + (250-150)*0.25 = 150 + 25 = 175
    }

    // === Easing Function Tests ===

    #[test]
    fn test_ease_in_out_cubic_start() {
        let result = ease_in_out_cubic(0.0);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ease_in_out_cubic_end() {
        let result = ease_in_out_cubic(1.0);
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ease_in_out_cubic_midpoint() {
        let result = ease_in_out_cubic(0.5);
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ease_in_out_cubic_first_half() {
        // In first half (t < 0.5), should use 4*t^3
        let result = ease_in_out_cubic(0.25);
        let expected = 4.0 * 0.25 * 0.25 * 0.25; // 4 * 0.015625 = 0.0625
        assert!((result - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ease_in_out_cubic_second_half() {
        // In second half (t >= 0.5), should use 1 - 4*(1-t)^3
        let result = ease_in_out_cubic(0.75);
        let expected = 1.0 - 4.0 * 0.25 * 0.25 * 0.25; // 1 - 0.0625 = 0.9375
        assert!((result - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ease_in_out_cubic_monotonic() {
        // Function should be monotonically increasing
        let values: Vec<f64> = (0..=10)
            .map(|i| ease_in_out_cubic(i as f64 / 10.0))
            .collect();
        for i in 1..values.len() {
            assert!(
                values[i] >= values[i - 1],
                "Easing function should be monotonically increasing"
            );
        }
    }

    // === Animator Tests ===

    #[test]
    fn test_animator_new() {
        let animator = Animator::new(0.0f32, 10.0f32, linear);
        assert_eq!(animator.from, 0.0);
        assert_eq!(animator.to, 10.0);
        assert_eq!(animator.duration, 0);
        assert_eq!(animator.animating, false);
        assert_eq!(*animator.current_value(), 0.0);
    }

    #[test]
    fn test_animator_start_from() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        animator.start_from(5.0, 15.0, 1000);

        assert_eq!(animator.from, 5.0);
        assert_eq!(animator.to, 15.0);
        assert_eq!(animator.duration, 1000);
        assert_eq!(animator.animating, true);
        assert_eq!(*animator.current_value(), 5.0);
    }

    #[test]
    fn test_animator_start() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        animator.last_value = 3.0; // Simulate current position
        animator.start(7.0, 500);

        assert_eq!(animator.from, 3.0); // Should start from current position
        assert_eq!(animator.to, 7.0);
        assert_eq!(animator.duration, 500);
        assert_eq!(animator.animating, true);
    }

    #[test]
    fn test_animator_zero_duration() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        animator.start(5.0, 0); // Zero duration

        let result = animator.update();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 5.0); // Should immediately be at target
        assert_eq!(animator.animating, false);
    }

    #[test]
    fn test_animator_not_animating() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        // Don't start animation
        let result = animator.update();
        assert!(result.is_none());
    }

    #[test]
    fn test_animator_is_animating() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        assert!(!animator.is_animating());

        animator.start(5.0, 1000);
        assert!(animator.is_animating());

        // Simulate finishing animation
        animator.animating = false;
        assert!(!animator.is_animating());
    }

    #[test]
    fn test_animator_bounds() {
        let start_bounds = Bounds::new(0, 0, 100, 100);
        let end_bounds = Bounds::new(100, 100, 200, 200);

        let mut animator = Animator::new(start_bounds, end_bounds.clone(), linear);
        animator.start(end_bounds, 0); // Zero duration for immediate completion

        let result = animator.update();
        assert!(result.is_some());
        let final_bounds = result.unwrap();
        assert_eq!(final_bounds.position.x, 100);
        assert_eq!(final_bounds.position.y, 100);
        assert_eq!(final_bounds.size.width, 200);
        assert_eq!(final_bounds.size.height, 200);
    }

    #[test]
    fn test_animator_with_easing() {
        let mut animator = Animator::new(0.0f64, 1.0f64, ease_in_out_cubic);
        animator.start_from(0.0, 1.0, 1000);

        // Manually set elapsed time to halfway point
        let start_time = Instant::now() - Duration::from_millis(500);
        animator.start_time = Some(start_time);

        let result = animator.update();
        assert!(result.is_some());

        // At t=0.5, ease_in_out_cubic should return 0.5
        let value = result.unwrap();
        assert!((value - 0.5).abs() < 0.01); // Allow small floating point error
    }

    #[test]
    fn test_animator_frame_tracking() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        animator.start(5.0, 1000);

        // Frame times should be empty initially
        assert_eq!(animator.frame_times.len(), 0);

        // First update should add frame time
        animator.update();
        assert_eq!(animator.frame_times.len(), 1);

        // Additional updates should add more frame times
        animator.update();
        assert_eq!(animator.frame_times.len(), 2);
    }

    #[test]
    fn test_animator_frame_tracking_limit() {
        let mut animator = Animator::new(0.0f32, 10.0f32, linear);
        animator.start(5.0, 1000);

        // Add more than 60 frame times
        for _ in 0..70 {
            animator.update();
        }

        // Should be limited to 60 frames
        assert_eq!(animator.frame_times.len(), 60);
    }

    #[test]
    fn test_linear_easing() {
        // Test our helper linear function
        assert_eq!(linear(0.0), 0.0);
        assert_eq!(linear(0.5), 0.5);
        assert_eq!(linear(1.0), 1.0);
        assert_eq!(linear(0.25), 0.25);
    }

    #[test]
    fn test_interpolatable_edge_cases() {
        // Test edge cases for different types

        // f32 with same values
        let result = 5.0f32.interpolate(&5.0f32, 0.5);
        assert_eq!(result, 5.0);

        // u8 with same values
        let result = 100u8.interpolate(&100u8, 0.5);
        assert_eq!(result, 100);

        // Bounds with same values
        let bounds = Bounds::new(10, 20, 100, 200);
        let result = bounds.interpolate(&bounds, 0.5);
        assert_eq!(result.position.x, 10);
        assert_eq!(result.position.y, 20);
        assert_eq!(result.size.width, 100);
        assert_eq!(result.size.height, 200);
    }

    #[test]
    fn test_negative_bounds_interpolation() {
        let start = Bounds::new(-100, -200, 50, 75);
        let end = Bounds::new(100, 200, 150, 225);
        let result = start.interpolate(&end, 0.5);

        assert_eq!(result.position.x, 0); // (-100 + 100) / 2
        assert_eq!(result.position.y, 0); // (-200 + 200) / 2
        assert_eq!(result.size.width, 100); // (50 + 150) / 2
        assert_eq!(result.size.height, 150); // (75 + 225) / 2
    }
}
