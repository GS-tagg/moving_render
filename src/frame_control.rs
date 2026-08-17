use std::time::{Duration, Instant};
use std::thread::sleep;

pub struct FpsTracker {
    target_fps: f32,
    frame_duration: Duration,
    fps_counter: u32,
    fps_timer: Instant,
    next_frame_time: Instant,
}

impl FpsTracker {
    pub fn new(target_fps: f32) -> Self {
        let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
        let now = Instant::now();
        Self {
            target_fps,
            frame_duration,
            fps_counter: 0,
            fps_timer: now,
            next_frame_time: now + frame_duration,
        }
    }

    // Call in loop 
    pub fn tick(&mut self) {
        let now = Instant::now();

        // Frame Limiter
        if now < self.next_frame_time {
            sleep(self.next_frame_time - now);
        }
        // Advance next frame target (prevents drift over time)
        self.next_frame_time += self.frame_duration;

        // Profiler/FPS Counter
        self.fps_counter += 1;
        if self.fps_timer.elapsed() >= Duration::from_secs(1) {
            println!("Actual FPS: {}", self.fps_counter);
            self.fps_counter = 0;
            self.fps_timer = Instant::now();
        }
    }
}