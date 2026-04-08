use std::f32::consts::TAU;

use crate::engine::uniform::{Uniform, UniformData};
use bytemuck::{Pod, Zeroable};
use derive_more::From;
use nalgebra::{Matrix4, Point3, Vector3, point, vector};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, From)]
pub struct MVP([[f32; 4]; 4]);

pub type CameraUniform = Uniform<MVP>;
impl UniformData for MVP {}

impl From<&Camera> for MVP {
    fn from(camera: &Camera) -> Self {
        Self(camera.build_view_projection_matrix().into())
    }
}

pub struct Camera {
    pub eye: Point3<f32>,
    pitch: f32, // rotation verticale
    yaw: f32,   // rotation horizontale
    up: Vector3<f32>,
    projection: Projection,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        let eye = point![250., 350., 200.];
        let znear = 0.1;
        let zfar = 1000.0;
        Self {
            eye,
            pitch: -0.82,
            yaw: -2.53,
            up: vector![0., 1., 0.],
            projection: Projection::new(width, height, 45_f32.to_radians(), znear, zfar),
        }
    }

    pub fn create_uniform(&self, device: &wgpu::Device, binding: u32) -> CameraUniform {
        MVP::from(self).create_uniform(device, binding, Some("camera_uniform"))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    pub fn forward(&self) -> Vector3<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        vector![cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch]
    }

    fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let target = self.eye + self.forward();
        let view = Matrix4::look_at_rh(&self.eye, &target, &self.up);
        let projection = self.projection.calc_matrix();
        projection * view
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    matrix: Matrix4<f32>,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        let aspect = width as f32 / height as f32;
        Self {
            aspect,
            fovy,
            znear,
            zfar,
            matrix: Matrix4::new_perspective(aspect, fovy, znear, zfar),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
        self.matrix = Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar);
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        self.matrix
    }
}

pub struct CameraController {
    speed: f32,
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    dx: f32,
    dy: f32,
    scroll: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            speed: 50.,
            amount_left: 0.,
            amount_right: 0.,
            amount_forward: 0.,
            amount_backward: 0.,
            amount_up: 0.,
            amount_down: 0.,
            dx: 0.0,
            dy: 0.0,
            scroll: 0.0,
            sensitivity: 0.1,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, state: ElementState) {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
            }
            KeyCode::Space => {
                self.amount_up = amount;
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
            }
            KeyCode::ControlLeft => {
                self.amount_down = amount;
            }
            _ => {}
        }
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        self.dx = dx;
        self.dy = dy;
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.eye += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.eye += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward =
            Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.eye += scrollward * self.scroll * self.speed * dt;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.eye.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += self.dx * self.sensitivity * dt;
        camera.pitch -= self.dy * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non-cardinal direction.
        self.dx = 0.0;
        self.dy = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -SAFE_FRAC_PI_2 {
            camera.pitch = -SAFE_FRAC_PI_2;
        } else if camera.pitch > SAFE_FRAC_PI_2 {
            camera.pitch = SAFE_FRAC_PI_2;
        }

        camera.yaw %= TAU;
    }
}
