use crate::calibration::Calibration;
use cgmath::*;
use joycon_sys::*;

#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub last_delta_rotation: Euler<Deg<f64>>,
    pub rotation: Quaternion<f64>,
    pub accel: Vector3<f64>,
    pub speed: Vector3<f64>,
    pub position: Vector3<f64>,
}

impl Position {
    pub fn new() -> Self {
        let zero = Euler::new(Deg(0.), Deg(0.), Deg(0.));
        Self {
            last_delta_rotation: zero,
            rotation: Quaternion::from(zero),
            accel: Vector3::zero(),
            speed: Vector3::zero(),
            position: Vector3::zero(),
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
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
        Handler {
            imu_cb: None,
            calib_gyro: Calibration::with_capacity(200),
            gyro_sens,
            calib_accel: Calibration::with_capacity(200),
            accel_sens,
            factory_calibration: spi::SensorCalibration::default(),
            user_calibration: spi::UserSensorCalibration::default(),
            position: Position::new(),
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

    pub fn position(&self) -> &Position {
        &self.position
    }

    fn acc_calib(&self) -> Vector3<f64> {
        self.user_calibration
            .acc_offset()
            .unwrap_or_else(|| self.factory_calibration.acc_offset())
    }

    fn gyro_calib(&self) -> Vector3<f64> {
        self.user_calibration
            .gyro_offset()
            .unwrap_or_else(|| self.factory_calibration.gyro_offset())
    }

    pub fn handle_frames(&mut self, frames: &[imu::Frame]) {
        let gyro_offset = self.gyro_calib();
        let acc_offset = self.acc_calib();
        for frame in frames.iter().rev() {
            let raw_delta_rotation = frame.rotation(gyro_offset, self.gyro_sens);
            // TODO: define axis and make sure it's accurate
            let raw_delta_rotation = Euler {
                x: raw_delta_rotation.y,
                y: -raw_delta_rotation.z,
                z: -raw_delta_rotation.x,
            };
            let raw_acc = frame.accel_g(acc_offset, self.accel_sens);
            if self.calib_nb > 0 {
                self.calib_gyro.push(Vector3::new(
                    raw_delta_rotation.x.0,
                    raw_delta_rotation.y.0,
                    raw_delta_rotation.z.0,
                ));
                self.calib_accel.push(raw_acc);
                self.calib_nb -= 1;
            }
            let c = self.calib_gyro.get_average();
            let delta_rotation = Euler::new(
                raw_delta_rotation.x - Deg(c.x),
                raw_delta_rotation.y - Deg(c.y),
                raw_delta_rotation.z - Deg(c.z),
            );
            self.position.last_delta_rotation = delta_rotation;
            self.position.rotation = self.position.rotation * Quaternion::from(delta_rotation);
            self.position.accel = raw_acc - self.calib_accel.get_average();

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
