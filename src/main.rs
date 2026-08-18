use std::num::NonZeroU32;
use std::rc::Rc;

use rayon::prelude::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod frame_control;
use frame_control::FpsTracker;

const RADIUS_EXTENT: f32 = 0.8;
const HALF_DEPTH: f32 = 0.4;

const MAX_RAY_STEPS: usize = 32;
const MAX_DISTANCE: f32 = 20.0;
const HIT_DISTANCE: f32 = 0.001;

const COS_30: f32 = 0.8660254;

// 3D vector struct
#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    #[inline]
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    #[inline]
    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    fn normalize(self) -> Self {
        let len_squared = self.dot(self);

        if len_squared > 0.0 {
            let inv_len = len_squared.sqrt().recip();

            Self::new(self.x * inv_len, self.y * inv_len, self.z * inv_len)
        } else {
            Self::new(0.0, 0.0, 0.0)
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

// Signed distance function for a triangular prism.
//
// - point: point in space
// - radius_extent: size of the triangular face
// - half_depth: half length along Z-axis
#[inline]
fn sd_tri_prism(point: Vec3, radius_extent: f32, half_depth: f32) -> f32 {
    let q = point.abs();

    let dist_z = q.z - half_depth;

    let dist_tri = (q.x * COS_30 + q.y * 0.5).max(-q.y) - radius_extent * 0.5;

    dist_z.max(dist_tri)
}

// Estimates the perpendicular surface direction using finite differences.
#[inline]
fn get_normal(point: Vec3, radius_extent: f32, half_depth: f32) -> Vec3 {
    const E: f32 = 0.001;

    let k0 = Vec3::new(1.0, -1.0, -1.0);
    let k1 = Vec3::new(-1.0, -1.0, 1.0);
    let k2 = Vec3::new(-1.0, 1.0, -1.0);
    let k3 = Vec3::new(1.0, 1.0, 1.0);

    let d0 = sd_tri_prism(point + k0 * E, radius_extent, half_depth);

    let d1 = sd_tri_prism(point + k1 * E, radius_extent, half_depth);

    let d2 = sd_tri_prism(point + k2 * E, radius_extent, half_depth);

    let d3 = sd_tri_prism(point + k3 * E, radius_extent, half_depth);

    let normal = k0 * d0 + k1 * d1 + k2 * d2 + k3 * d3;

    normal.normalize()
}

// Raymarches from ray_origin along ray_dir.
#[inline]
fn raymarch(
    ray_origin: Vec3,
    ray_dir: Vec3,
    radius_extent: f32,
    half_depth: f32,
) -> Option<(Vec3, Vec3)> {
    let mut total_dist = 0.0;

    for _ in 0..MAX_RAY_STEPS {
        // Don't bother evaluating the SDF if we're already too far away.
        if total_dist > MAX_DISTANCE {
            break;
        }
        let current_pos = ray_origin + ray_dir * total_dist;

        let dist = sd_tri_prism(current_pos, radius_extent, half_depth);
        // Surface hit.
        if dist < HIT_DISTANCE {
            let surface_normal = get_normal(current_pos, radius_extent, half_depth);
            return Some((current_pos, surface_normal));
        }
        total_dist += dist;
    }
    None
}

// Rotates a vector around the Y axis.
#[inline]
fn rotate_y(vector: Vec3, cos_angle: f32, sin_angle: f32) -> Vec3 {
    Vec3::new(
        vector.x * cos_angle - vector.z * sin_angle,
        vector.y,
        vector.x * sin_angle + vector.z * cos_angle,
    )
}

// Generates one normalized camera ray for every pixel.
// This is done when the window size changes rather than every frame; as was done before.
fn generate_ray_directions(width: usize, height: usize) -> Vec<Vec3> {
    let mut rays = Vec::with_capacity(width * height);

    let width_half = width as f32 * 0.5;
    let height_half = height as f32 * 0.5;
    let height_f = height as f32;

    for pixel_y in 0..height {
        for pixel_x in 0..width {
            let uv_x = (pixel_x as f32 - width_half) / height_f;

            let uv_y = -(pixel_y as f32 - height_half) / height_f;

            rays.push(Vec3::new(uv_x, uv_y, 1.0).normalize());
        }
    }
    rays
}

fn main() {
    let mut tracker = FpsTracker::new(60.0);

    let event_loop = EventLoop::new().unwrap();

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("3D Triangular Prism")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)
            .unwrap(),
    );

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    let initial_size = window.inner_size();
    let mut width = initial_size.width as usize;
    let mut height = initial_size.height as usize;

    // Generate the camera rays once.
    //to remove the overhead of doing this every frame
    let mut ray_directions = generate_ray_directions(width, height);
    let mut rotation_angle = 0.0f32;

    // The light never changes, so normalize it only once.
    let light_world = Vec3::new(0.5, 1.0, -0.5).normalize();

    // Camera position before rotation.
    let base_ray_origin = Vec3::new(0.0, 0.0, -3.0);

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }

                        WindowEvent::Resized(size) => {
                            if let (Some(w), Some(h)) =
                                (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                            {
                                surface.resize(w, h).unwrap();

                                width = size.width as usize;

                                height = size.height as usize;

                                // Rebuild the ray cache because
                                // the camera projection changed.
                                ray_directions = generate_ray_directions(width, height);
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            let render_start = tracker.begin_render();

                            let mut buffer = surface.buffer_mut().unwrap();

                            // Advance rotation.
                            rotation_angle += 0.02;

                            let cos_angle = rotation_angle.cos();
                            let sin_angle = rotation_angle.sin();

                            // Rotate the light once per frame.
                            let light_direction = rotate_y(light_world, cos_angle, sin_angle);

                            // Rotate the camera origin once per frame.
                            let rotated_origin = rotate_y(base_ray_origin, cos_angle, sin_angle);

                            // Rayon deistributes the work across multiple threads
                            // this is possible because each pixel is independant
                            buffer
                                .chunks_exact_mut(width)
                                .enumerate()
                                .par_bridge()
                                .for_each(|(pixel_y, row)| {
                                    let ray_start = pixel_y * width;

                                    for (pixel_x, pixel) in row.iter_mut().enumerate() {
                                        let base_ray_dir = ray_directions[ray_start + pixel_x];

                                        // Rotate the precomputed
                                        // camera ray.
                                        let rotated_dir =
                                            rotate_y(base_ray_dir, cos_angle, sin_angle);

                                        let pixel_color =
                                            if let Some((_surface_pos, surface_normal)) = raymarch(
                                                rotated_origin,
                                                rotated_dir,
                                                RADIUS_EXTENT,
                                                HALF_DEPTH,
                                            ) {
                                                let diffuse_factor =
                                                    surface_normal.dot(light_direction).max(0.1);

                                                let intensity = (diffuse_factor * 255.0) as u32;

                                                (intensity << 16) | (intensity << 8) | intensity
                                            } else {
                                                0xFF000000
                                            };

                                        *pixel = pixel_color;
                                    }
                                });

                            buffer.present().unwrap();

                            tracker.end_render(render_start);

                            tracker.tick();

                            window.request_redraw();
                        }

                        _ => {}
                    }
                }

                _ => {}
            }
        })
        .unwrap();
}
