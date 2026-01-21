use nalgebra::{Matrix4, Point3, Vector3, point, vector};
use winit::keyboard::KeyCode;

pub struct Camera {
    eye: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
    pub aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            eye: point![0., 0., -3.],
            target: point![0., 0., 0.],
            up: vector![0., 1., 0.],
            aspect: width as f32 / height as f32,
            fovy: 45.0_f32.to_radians(),
            znear: 0.001,
            zfar: 10000.0,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(&self.eye, &self.target, &self.up);
        let proj: nalgebra::Matrix<
            f32,
            nalgebra::Const<4>,
            nalgebra::Const<4>,
            nalgebra::ArrayStorage<f32, 4, 4>,
        > = Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar);
        return proj * view;
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_down_pressed: bool,
    is_up_pressed: bool,
    is_boost_pressed: bool,
    yaw: f32,   // rotation horizontale
    pitch: f32, // rotation verticale
    sensitivity: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            speed: 0.005,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_down_pressed: false,
            is_up_pressed: false,
            is_boost_pressed: false,
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: 0.001,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
            }
            KeyCode::Space => {
                self.is_up_pressed = is_pressed;
            }
            KeyCode::ShiftLeft => {
                self.is_down_pressed = is_pressed;
            }
            KeyCode::ControlLeft => {
                self.is_boost_pressed = is_pressed;
            }
            _ => {}
        }
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.sensitivity;
        self.pitch -= dy * self.sensitivity;

        let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01;
        if self.pitch > pitch_limit {
            self.pitch = pitch_limit;
        }
        if self.pitch < -pitch_limit {
            self.pitch = -pitch_limit;
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let forward = vector![
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos()
        ]
        .normalize();

        let right = forward.cross(&camera.up).normalize();
        let up = right.cross(&forward).normalize();

        let mut velocity = self.speed;
        if self.is_boost_pressed {
            velocity *= 5.0;
        }

        let horizontal_forward = Vector3::new(forward.x, 0.0, forward.z).normalize();

        if self.is_forward_pressed {
            camera.eye += horizontal_forward * velocity;
        }
        if self.is_backward_pressed {
            camera.eye -= horizontal_forward * velocity;
        }
        if self.is_right_pressed {
            camera.eye += right * velocity;
        }
        if self.is_left_pressed {
            camera.eye -= right * velocity;
        }
        if self.is_up_pressed {
            camera.eye += up * velocity;
        }
        if self.is_down_pressed {
            camera.eye -= up * velocity;
        }

        camera.target = camera.eye + forward;
    }
}
