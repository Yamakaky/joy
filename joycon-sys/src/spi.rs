use crate::common::*;
use byteorder::{ByteOrder, LittleEndian};
use cgmath::Vector3;
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SPIRange(u32, u8);

pub const RANGE_FACTORY_CALIBRATION_SENSORS: SPIRange = SPIRange(0x6020, 0x18);
pub const RANGE_FACTORY_CALIBRATION_STICKS: SPIRange = SPIRange(0x603D, 0x12);
pub const RANGE_USER_CALIBRATION_STICKS: SPIRange = SPIRange(0x8010, 0x16);
pub const RANGE_USER_CALIBRATION_SENSORS: SPIRange = SPIRange(0x8026, 0x1A);

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SPIReadRequest {
    offset: [u8; 4],
    size: u8,
}

impl SPIReadRequest {
    pub fn new(range: SPIRange) -> SPIReadRequest {
        assert!(range.1 <= 0x1d);
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, range.0);
        SPIReadRequest {
            offset: buf,
            size: range.1,
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SPIWriteRequest {
    address: [u8; 4],
    size: u8,
    data: SPIData,
}

impl From<ControllerColor> for SPIWriteRequest {
    fn from(color: ControllerColor) -> SPIWriteRequest {
        let range = RANGE_CONTROLLER_COLOR;
        assert!(range.1 <= 0x1d);
        SPIWriteRequest {
            address: range.0.to_le_bytes(),
            size: range.1,
            data: SPIData { color },
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SPIReadResult {
    address: [u8; 4],
    size: u8,
    data: SPIData,
}

impl SPIReadResult {
    pub fn range(&self) -> SPIRange {
        SPIRange(LittleEndian::read_u32(&self.address), self.size)
    }

    pub fn sticks_factory_calib(&self) -> Option<&SticksCalibration> {
        if self.range() == RANGE_FACTORY_CALIBRATION_STICKS {
            Some(unsafe { &self.data.sticks_factory_calib })
        } else {
            None
        }
    }

    pub fn sticks_user_calib(&self) -> Option<&UserSticksCalibration> {
        if self.range() == RANGE_USER_CALIBRATION_STICKS {
            Some(unsafe { &self.data.sticks_user_calib })
        } else {
            None
        }
    }
    pub fn imu_factory_calib(&self) -> Option<&SensorCalibration> {
        if self.range() == RANGE_FACTORY_CALIBRATION_SENSORS {
            Some(unsafe { &self.data.imu_factory_calib })
        } else {
            None
        }
    }

    pub fn imu_user_calib(&self) -> Option<&UserSensorCalibration> {
        if self.range() == RANGE_USER_CALIBRATION_SENSORS {
            Some(unsafe { &self.data.imu_user_calib })
        } else {
            None
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SPIWriteResult {
    status: u8,
}

impl SPIWriteResult {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union SPIData {
    sticks_factory_calib: SticksCalibration,
    sticks_user_calib: UserSticksCalibration,
    imu_factory_calib: SensorCalibration,
    imu_user_calib: UserSensorCalibration,
}

impl fmt::Debug for SPIData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SPIResultData").finish()
    }
}

// TODO: clean
#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct SticksCalibration {
    pub left: LeftStickCalibration,
    pub right: RightStickCalibration,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct UserSticksCalibration {
    pub left: UserStickCalibration,
    pub right: UserStickCalibration,
}

#[repr(packed)]
#[derive(Copy, Clone, Default)]
pub struct LeftStickCalibration {
    max: [u8; 3],
    center: [u8; 3],
    min: [u8; 3],
}

impl LeftStickCalibration {
    fn conv_x(&self, raw: [u8; 3]) -> u16 {
        (((raw[1] as u16) << 8) & 0xF00) | raw[0] as u16
    }

    fn conv_y(&self, raw: [u8; 3]) -> u16 {
        ((raw[2] as u16) << 4) | (raw[1] >> 4) as u16
    }

    pub fn max(&self) -> (u16, u16) {
        let center = self.center();
        (
            (center.0 + self.conv_x(self.max)).min(0xFFF),
            (center.1 + self.conv_y(self.max)).min(0xFFF),
        )
    }

    pub fn center(&self) -> (u16, u16) {
        (self.conv_x(self.center), self.conv_y(self.center))
    }

    pub fn min(&self) -> (u16, u16) {
        let center = self.center();
        (
            center.0.saturating_sub(self.conv_x(self.min)),
            center.1.saturating_sub(self.conv_y(self.min)),
        )
    }

    pub fn value_from_raw(&self, x: u16, y: u16) -> (f64, f64) {
        let min = self.min();
        let center = self.center();
        let max = self.max();
        let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
        let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

        (
            if rel_x >= 0. {
                rel_x / (max.0 as f64 - center.0 as f64)
            } else {
                rel_x / (center.0 as f64 - min.0 as f64)
            },
            if rel_y >= 0. {
                rel_y / (max.1 as f64 - center.1 as f64)
            } else {
                rel_y / (center.1 as f64 - min.1 as f64)
            },
        )
    }
}

impl fmt::Debug for LeftStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StickCalibration")
            .field("min", &self.min())
            .field("center", &self.center())
            .field("max", &self.max())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Default)]
pub struct RightStickCalibration {
    center: [u8; 3],
    min: [u8; 3],
    max: [u8; 3],
}

impl RightStickCalibration {
    fn conv_x(&self, raw: [u8; 3]) -> u16 {
        (((raw[1] as u16) << 8) & 0xF00) | raw[0] as u16
    }

    fn conv_y(&self, raw: [u8; 3]) -> u16 {
        ((raw[2] as u16) << 4) | (raw[1] >> 4) as u16
    }

    pub fn max(&self) -> (u16, u16) {
        let center = self.center();
        (
            (center.0 + self.conv_x(self.max)).min(0xFFF),
            (center.1 + self.conv_y(self.max)).min(0xFFF),
        )
    }

    pub fn center(&self) -> (u16, u16) {
        (self.conv_x(self.center), self.conv_y(self.center))
    }

    pub fn min(&self) -> (u16, u16) {
        let center = self.center();
        (
            center.0.saturating_sub(self.conv_x(self.min)),
            center.1.saturating_sub(self.conv_y(self.min)),
        )
    }

    pub fn value_from_raw(&self, x: u16, y: u16) -> (f64, f64) {
        let min = self.min();
        let center = self.center();
        let max = self.max();
        let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
        let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

        (
            if rel_x >= 0. {
                rel_x / (max.0 as f64 - center.0 as f64)
            } else {
                rel_x / (center.0 as f64 - min.0 as f64)
            },
            if rel_y >= 0. {
                rel_y / (max.1 as f64 - center.1 as f64)
            } else {
                rel_y / (center.1 as f64 - min.1 as f64)
            },
        )
    }
}

impl fmt::Debug for RightStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StickCalibration")
            .field("min", &self.min())
            .field("center", &self.center())
            .field("max", &self.max())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct UserStickCalibration {
    magic: [u8; 2],
    // TODO: left and right are different
    calib: LeftStickCalibration,
}

impl UserStickCalibration {
    pub fn calib(&self) -> Option<LeftStickCalibration> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib)
        } else {
            None
        }
    }

    pub fn max(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.max())
        } else {
            None
        }
    }

    pub fn center(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.center())
        } else {
            None
        }
    }

    pub fn min(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.min())
        } else {
            None
        }
    }
}

impl fmt::Debug for UserStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.magic == USER_CALIB_MAGIC {
            f.write_fmt(format_args!("{:?}", self.calib))
        } else {
            f.write_str("NoUserStickCalibration")
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct SensorCalibration {
    acc_orig: [I16LE; 3],
    acc_sens: [I16LE; 3],
    gyro_orig: [I16LE; 3],
    gyro_sens: [I16LE; 3],
}

impl SensorCalibration {
    pub fn acc_offset(&self) -> Vector3<f64> {
        vector_from_raw(self.acc_orig)
    }

    pub fn acc_factor(&self) -> Vector3<f64> {
        vector_from_raw(self.acc_sens)
    }

    pub fn gyro_offset(&self) -> Vector3<f64> {
        vector_from_raw(self.gyro_orig)
    }

    pub fn gyro_factor(&self) -> Vector3<f64> {
        vector_from_raw(self.gyro_sens)
    }
}

const USER_CALIB_MAGIC: [u8; 2] = [0xB2, 0xA1];

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct UserSensorCalibration {
    magic: [u8; 2],
    calib: SensorCalibration,
}

impl UserSensorCalibration {
    pub fn calib(&self) -> Option<SensorCalibration> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib)
        } else {
            None
        }
    }
    pub fn acc_offset(&self) -> Option<Vector3<f64>> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.acc_offset())
        } else {
            None
        }
    }

    pub fn acc_factor(&self) -> Option<Vector3<f64>> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.acc_factor())
        } else {
            None
        }
    }

    pub fn gyro_offset(&self) -> Option<Vector3<f64>> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.gyro_offset())
        } else {
            None
        }
    }

    pub fn gyro_factor(&self) -> Option<Vector3<f64>> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.gyro_factor())
        } else {
            None
        }
    }
}
