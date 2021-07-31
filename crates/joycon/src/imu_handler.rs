use crate::calibration::Calibration;
use cgmath::*;
use input::WhichController;
use joycon_sys::*;

/// Acceleration and gyroscope data for the controller.
///
/// <https://camo.githubusercontent.com/3e980a22532232f4b28ddbea4f119e55c7025ddb52efdaffb96a22f06cb203b3/687474703a2f2f6374636165722e636f6d2f7769692f7377697463682f6a6f79636f6e5f6163632d6779726f5f7269676874322e706e67>
#[derive(Debug, Copy, Clone)]
pub struct IMU {
    /// Current rotation speed.
    ///
    /// Yaw, pitch, roll in this order. Unit in degree per second (dps).
    pub gyro: Vector3<f64>,
    /// Current acceleration.
    pub accel: Vector3<f64>,
}

impl IMU {
    pub const SAMPLE_DURATION: f64 = imu::IMU_SAMPLE_DURATION;
    pub const SAMPLE_PER_SECOND: u32 = imu::IMU_SAMPLES_PER_SECOND;
}

pub struct Handler {
    device_type: WhichController,
    calib_gyro: Calibration,
    gyro_sens: imu::GyroSens,
    accel_sens: imu::AccSens,
    factory_calibration: spi::SensorCalibration,
    user_calibration: spi::UserSensorCalibration,
    calib_nb: u32,
}

impl Handler {
    pub fn new(
        device_type: WhichController,
        gyro_sens: imu::GyroSens,
        accel_sens: imu::AccSens,
    ) -> Self {
        Handler {
            device_type,
            calib_gyro: Calibration::with_capacity(200),
            gyro_sens,
            accel_sens,
            factory_calibration: spi::SensorCalibration::default(),
            user_calibration: spi::UserSensorCalibration::default(),
            calib_nb: 0,
        }
    }

    pub fn set_factory(&mut self, calib: spi::SensorCalibration) {
        self.factory_calibration = calib;
    }

    pub fn set_user(&mut self, calib: spi::UserSensorCalibration) {
        self.user_calibration = calib;
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

    pub fn handle_frames(&mut self, frames: &[imu::Frame]) -> [IMU; 3] {
        let gyro_offset = self.gyro_calib();
        let acc_offset = self.acc_calib();
        let mut out = [IMU {
            gyro: Vector3::zero(),
            accel: Vector3::zero(),
        }; 3];
        for (frame, out) in frames.iter().rev().zip(out.iter_mut()) {
            let raw_rotation = frame.rotation_dps(gyro_offset, self.gyro_sens);
            let raw_acc = frame.accel_g(acc_offset, self.accel_sens);
            if self.calib_nb > 0 {
                self.calib_gyro.push(raw_rotation);
                self.calib_nb -= 1;
            }
            *out = IMU {
                gyro: raw_rotation - self.calib_gyro.get_average(),
                accel: raw_acc,
            };
            // The devices don't have the same axis.
            match self.device_type {
                WhichController::LeftJoyCon | WhichController::ProController => {
                    out.gyro.y = -out.gyro.y;
                    out.gyro.z = -out.gyro.z;
                    out.accel.x = -out.accel.x;
                }
                WhichController::RightJoyCon => {
                    out.accel = -out.accel;
                }
            }
        }
        out
    }

    pub fn reset_calibration(&mut self) {
        self.calib_gyro.reset();
        self.calib_nb = 0;
    }
}
