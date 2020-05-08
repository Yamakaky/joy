use crate::input::Vector3;
use byteorder::{ByteOrder, LittleEndian};
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SPIRange(u32, u8);

pub const RANGE_FACTORY_CALIBRATION_SENSORS: SPIRange = SPIRange(0x6020, 0x18);
pub const RANGE_FACTORY_CALIBRATION_STICKS: SPIRange = SPIRange(0x603D, 0x12);
pub const RANGE_USER_CALIBRATION_STICKS: SPIRange = SPIRange(0x8010, 0x16);
pub const RANGE_USER_CALIBRATION_SENSORS: SPIRange = SPIRange(0x8026, 0x1A);

#[repr(C)]
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SPIReadResult {
    address: [u8; 4],
    size: u8,
    data: SPIResultData,
}

impl SPIReadResult {
    pub fn range(&self) -> SPIRange {
        SPIRange(LittleEndian::read_u32(&self.address), self.size)
    }

    pub fn factory_calib(&self) -> Option<&SensorCalibration> {
        if self.range() == RANGE_FACTORY_CALIBRATION_SENSORS {
            Some(unsafe { &self.data.factory_calib })
        } else {
            None
        }
    }

    pub fn user_calib(&self) -> Option<&UserSensorCalibration> {
        if self.range() == RANGE_USER_CALIBRATION_SENSORS {
            Some(unsafe { &self.data.user_calib })
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union SPIResultData {
    factory_calib: SensorCalibration,
    user_calib: UserSensorCalibration,
}

impl fmt::Debug for SPIResultData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SPIResultData").finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SensorCalibration {
    acc_orig: [[u8; 2]; 3],
    acc_sens: [[u8; 2]; 3],
    gyro_orig: [[u8; 2]; 3],
    gyro_sens: [[u8; 2]; 3],
}

impl SensorCalibration {
    pub fn acc_offset(&self) -> Vector3 {
        Vector3::from_raw(self.acc_orig)
    }

    pub fn acc_factor(&self) -> Vector3 {
        Vector3::from_raw(self.acc_sens)
    }

    pub fn gyro_offset(&self) -> Vector3 {
        Vector3::from_raw(self.gyro_orig)
    }

    pub fn gyro_factor(&self) -> Vector3 {
        Vector3::from_raw(self.gyro_sens)
    }
}

const USER_SENSOR_CALIB_MAGIC: [u8; 2] = [0xB2, 0xA1];

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UserSensorCalibration {
    magic: [u8; 2],
    calib: SensorCalibration,
}

impl UserSensorCalibration {
    pub fn acc_offset(&self) -> Option<Vector3> {
        if self.magic == USER_SENSOR_CALIB_MAGIC {
            Some(self.calib.acc_offset())
        } else {
            None
        }
    }

    pub fn acc_factor(&self) -> Option<Vector3> {
        if self.magic == USER_SENSOR_CALIB_MAGIC {
            Some(self.calib.acc_factor())
        } else {
            None
        }
    }

    pub fn gyro_offset(&self) -> Option<Vector3> {
        if self.magic == USER_SENSOR_CALIB_MAGIC {
            Some(self.calib.gyro_offset())
        } else {
            None
        }
    }

    pub fn gyro_factor(&self) -> Option<Vector3> {
        if self.magic == USER_SENSOR_CALIB_MAGIC {
            Some(self.calib.gyro_factor())
        } else {
            None
        }
    }
}
