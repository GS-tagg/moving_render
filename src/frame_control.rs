use std::time::{Duration, Instant};
use std::thread::sleep;

pub struct FpsTracker {
    frame_duration: Duration,
    fps_counter: u32,
    fps_timer: Instant,
    next_frame_time: Instant,
    render_time_accum: Duration,
}

impl FpsTracker {
    pub fn new(target_fps: f32) -> Self {
        let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
        let now = Instant::now();
        Self {
            frame_duration,
            fps_counter: 0,
            fps_timer: now,
            next_frame_time: now + frame_duration,
            render_time_accum: Duration::ZERO,
        }
    }

    // Call this before render work
    pub fn begin_render(&self) -> Instant {
        Instant::now()
    }

    /// Call this right after render work
    pub fn end_render(&mut self, start: Instant) {
        self.render_time_accum += start.elapsed();
    }

    // Call in loop after end_render
    pub fn tick(&mut self) {
        let now = Instant::now();

        // Frame Limiter
        if now < self.next_frame_time {
            sleep(self.next_frame_time - now);
        }
        self.next_frame_time += self.frame_duration;

        self.fps_counter += 1;
        if self.fps_timer.elapsed() >= Duration::from_secs(1) {
            println!("Actual FPS: {}", self.fps_counter);
            println!(
                "Avg Render Time: {:.2} ms",
                self.render_time_accum.as_secs_f32() * 1000.0 / self.fps_counter as f32
            );
            self.fps_counter = 0;
            self.fps_timer = Instant::now();
            self.render_time_accum = Duration::ZERO;
        }
    }
}