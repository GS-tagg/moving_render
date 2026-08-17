use std::num::NonZeroU32;
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
mod frame_control;
use frame_control::FpsTracker;

fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn main() {
    let mut tracker = FpsTracker::new(60.0);
    let event_loop = EventLoop::new().unwrap();

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Circle Test")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)
            .unwrap(),
    );

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    // 1. Interpolation State
    let mut t = 0.0f32;
    let mut direction = 1.0f32; // 1.0 = right, -1.0 = left
    let speed = 0.5f32; // Complete one traversal across the screen in ~2 seconds
    let delta_time = 1.0f32 / 60.0f32; // Fixed delta time matching target 60 FPS

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    if let (Some(width), Some(height)) = (
                        NonZeroU32::new(size.width),
                        NonZeroU32::new(size.height),
                    ) {
                        surface.resize(width, height).unwrap();
                    }
                }
                WindowEvent::RedrawRequested => {
                    let render_start = tracker.begin_render();
                    let mut buffer = surface.buffer_mut().unwrap();
                    let size = window.inner_size();
                    let (width, height) = (size.width as i32, size.height as i32);

                    // 2. Advance progress 't' and ping-pong at bounds
                    t += speed * delta_time * direction;
                    if t >= 1.0 {
                        t = 1.0;
                        direction = -1.0;
                    } else if t <= 0.0 {
                        t = 0.0;
                        direction = 1.0;
                    }

                    buffer.fill(0xFF000000);

                    let radius = 100.0f32;
                    let radius_sq = radius * radius;

                    // 3. Interpolate center X between left edge and right edge padding
                    let start_x = radius + 20.0;
                    let end_x = width as f32 - radius - 20.0;
                    let center_x = interpolate(start_x, end_x, t);
                    let center_y = height as f32 / 2.0;

                    let min_x = (center_x - radius).floor().max(0.0) as i32;
                    let max_x = (center_x + radius).ceil().min(width as f32) as i32;
                    let min_y = (center_y - radius).floor().max(0.0) as i32;
                    let max_y = (center_y + radius).ceil().min(height as f32) as i32;

                    for y in min_y..max_y {
                        for x in min_x..max_x {
                            let dx = x as f32 - center_x;
                            let dy = y as f32 - center_y;

                            if dx * dx + dy * dy <= radius_sq {
                                buffer[(y * width + x) as usize] = 0x00FFFFFF;
                            }
                        }
                    }

                    buffer.present().unwrap();
                    tracker.end_render(render_start);
                    tracker.tick();
                    window.request_redraw();
                }
                _ => (),
            },
            _ => (),
        })
        .unwrap();
}