use crate::calibration::Calibration;
use cgmath::*;
use joycon_sys::*;

#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub last_delta_rotation: Euler<Deg<f32>>,
    pub rotation: Quaternion<f32>,
    pub speed: Vector3<f32>,
    pub position: Vector3<f32>,
}

pub struct Handler {
    imu_cb: Option<Box<dyn FnMut(&Position)>>,
    calib_gyro: Calibration,
    gyro_sens: imu::GyroSens,
    calib_accel: Calibration,
    accel_sens: imu::AccSens,
    factory_calibration: spi::SensorCalibration,
    user_calibration: spi::UserSensorCalibration,
    position: Position,
    calib_nb: u32,
}

impl Handler {
    pub fn new(gyro_sens: imu::GyroSens, accel_sens: imu::AccSens) -> Self {
        let zero = Euler::new(Deg(0.), Deg(0.), Deg(0.));
        Handler {
            imu_cb: None,
            calib_gyro: Calibration::with_capacity(200),
            gyro_sens,
            calib_accel: Calibration::with_capacity(200),
            accel_sens,
            factory_calibration: spi::SensorCalibration::default(),
            user_calibration: spi::UserSensorCalibration::default(),
            position: Position {
                last_delta_rotation: zero,
                rotation: Quaternion::from(zero),
                speed: Vector3::zero(),
                position: Vector3::zero(),
            },
            calib_nb: 0,
        }
    }

    pub fn set_factory(&mut self, calib: spi::SensorCalibration) {
        self.factory_calibration = calib;
    }

    pub fn set_user(&mut self, calib: spi::UserSensorCalibration) {
        self.user_calibration = calib;
    }

    pub fn set_cb(&mut self, cb: Box<dyn FnMut(&Position)>) {
        self.imu_cb = Some(cb);
    }

    fn gyro_calib(&self) -> Vector3<f32> {
        self.user_calibration
            .gyro_offset()
            .unwrap_or_else(|| self.factory_calibration.gyro_offset())
    }

    pub fn handle_frames(&mut self, frames: &[imu::Frame]) {
        let offset = self.gyro_calib();
        for frame in frames.iter().rev() {
            let raw_delta_rotation = frame.rotation(offset, self.gyro_sens);
            if self.calib_nb > 0 {
                self.calib_gyro.push(raw_delta_rotation);
                self.calib_nb -= 1;
            }
            let c = self.calib_gyro.get_average();
            let delta_rotation = Euler::new(
                raw_delta_rotation.x - c.x,
                raw_delta_rotation.y - c.y,
                raw_delta_rotation.z - c.z,
            );
            self.position.last_delta_rotation = delta_rotation;
            self.position.rotation = self.position.rotation * Quaternion::from(delta_rotation);
            if let Some(ref mut cb) = self.imu_cb {
                cb(&self.position);
            }
        }
    }

    pub fn reset_calibration(&mut self) {
        self.calib_gyro.reset();
        self.calib_nb = 200;
    }
}
