use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use winit::{dpi::PhysicalPosition, event::*};

#[rustfmt::skip] pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    pub pos: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<P, A, B>(pos: P, yaw: A, pitch: B) -> Self
    where
        P: Into<Point3<f32>>,
        A: Into<Rad<f32>>,
        B: Into<Rad<f32>>,
    {
        Self {
            pos: pos.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }
    pub fn view_mat(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        Matrix4::look_to_rh(
            self.pos,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw),
            Vector3::unit_y(),
        )
    }
}

#[derive(Debug)]
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn proj_mat(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct Controller {
    amount_right: f32,
    amount_left: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl Controller {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_right: 0.0,
            amount_left: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, dx: f64, dy: f64) {
        self.rotate_horizontal = dx as f32;
        self.rotate_vertical = dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 10.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };

        self.speed -= self.scroll;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        let mut displacement = forward * (self.amount_forward - self.amount_backward)
            + right * (self.amount_right - self.amount_left)
            + Vector3::unit_y() * (self.amount_up - self.amount_down);

        if !displacement.is_zero() {
            displacement = displacement.normalize();
        }

        camera.pos += displacement * dt * self.speed;

        // TODO: Scroll zoom stuff here
        self.scroll = 0.0;

        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }

        //print!("\x1B[2J\x1B[1;1H");
        //println!(
        //    "al: {:?} ar: {:?} af: {:?} ab: {:?} au: {:?} ad: {:?} rv: {:?} rh: {:?}",
        //    self.amount_left, self.amount_right,
        //    self.amount_forward, self.amount_backward,
        //    self.amount_up, self.amount_down,
        //    self.rotate_horizontal, self.rotate_vertical,
        //);
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }
}
