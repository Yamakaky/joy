use crate::{common::*, input::UseSPIColors};
use byteorder::{ByteOrder, LittleEndian};
use cgmath::{vec2, Vector2, Vector3};
use std::{convert::TryFrom, fmt, num::ParseIntError, str::FromStr};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SPIRange(u32, u8);

const RANGE_FACTORY_CALIBRATION_SENSORS: SPIRange = SPIRange(0x6020, 0x18);
const RANGE_FACTORY_CALIBRATION_STICKS: SPIRange = SPIRange(0x603D, 0x12);
const RANGE_USER_CALIBRATION_STICKS: SPIRange = SPIRange(0x8010, 0x16);
const RANGE_USER_CALIBRATION_SENSORS: SPIRange = SPIRange(0x8026, 0x1A);

const RANGE_CONTROLLER_COLOR_USE_SPI: SPIRange = SPIRange(0x601B, 1);
const RANGE_CONTROLLER_COLOR: SPIRange = SPIRange(0x6050, 12);

pub trait SPI: TryFrom<SPIReadResult, Error = WrongRangeError> {
    fn range() -> SPIRange;
}

#[derive(Debug, Clone, Copy)]
pub struct WrongRangeError {
    expected: SPIRange,
    got: SPIRange,
}

impl fmt::Display for WrongRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wrong SPI range: expected {:?}, got {:?}",
            self.expected, self.got
        )
    }
}

impl std::error::Error for WrongRangeError {}

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
        let range = ControllerColor::range();
        assert!(range.1 <= 0x1d);
        SPIWriteRequest {
            address: range.0.to_le_bytes(),
            size: range.1,
            data: SPIData { color },
        }
    }
}

impl SPI for UseSPIColors {
    fn range() -> SPIRange {
        RANGE_CONTROLLER_COLOR_USE_SPI
    }
}

impl From<UseSPIColors> for SPIWriteRequest {
    fn from(use_spi_colors: UseSPIColors) -> SPIWriteRequest {
        let range = UseSPIColors::range();
        assert!(range.1 <= 0x1d);
        SPIWriteRequest {
            address: range.0.to_le_bytes(),
            size: range.1,
            data: SPIData {
                use_spi_colors: use_spi_colors.into(),
            },
        }
    }
}

impl TryFrom<SPIReadResult> for UseSPIColors {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.use_spi_colors.try_into().unwrap() })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
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
    color: ControllerColor,
    use_spi_colors: RawId<UseSPIColors>,
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

impl SPI for SticksCalibration {
    fn range() -> SPIRange {
        RANGE_FACTORY_CALIBRATION_STICKS
    }
}

impl TryFrom<SPIReadResult> for SticksCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.sticks_factory_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
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

    pub fn value_from_raw(&self, x: u16, y: u16) -> Vector2<f64> {
        let min = self.min();
        let center = self.center();
        let max = self.max();
        let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
        let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

        vec2(
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

    pub fn value_from_raw(&self, x: u16, y: u16) -> Vector2<f64> {
        let min = self.min();
        let center = self.center();
        let max = self.max();
        let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
        let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

        vec2(
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
impl SPI for UserSticksCalibration {
    fn range() -> SPIRange {
        RANGE_USER_CALIBRATION_STICKS
    }
}

impl TryFrom<SPIReadResult> for UserSticksCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.sticks_user_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
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

    pub fn set_acc_offset(&mut self, offset: Vector3<f64>) {
        self.acc_orig = raw_from_vector(offset);
    }

    pub fn acc_factor(&self) -> Vector3<f64> {
        vector_from_raw(self.acc_sens)
    }

    pub fn set_acc_factor(&mut self, factor: Vector3<f64>) {
        self.acc_sens = raw_from_vector(factor);
    }

    pub fn gyro_offset(&self) -> Vector3<f64> {
        vector_from_raw(self.gyro_orig)
    }

    pub fn set_gyro_offset(&mut self, offset: Vector3<f64>) {
        self.gyro_orig = raw_from_vector(offset);
    }

    pub fn gyro_factor(&self) -> Vector3<f64> {
        vector_from_raw(self.gyro_sens)
    }

    pub fn set_gyro_factor(&mut self, factor: Vector3<f64>) {
        self.gyro_sens = raw_from_vector(factor);
}
}

impl SPI for SensorCalibration {
    fn range() -> SPIRange {
        RANGE_FACTORY_CALIBRATION_SENSORS
    }
}

impl TryFrom<SPIReadResult> for SensorCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.imu_factory_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

const USER_CALIB_MAGIC: [u8; 2] = [0xB2, 0xA1];

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct UserSensorCalibration {
    magic: [u8; 2],
    calib: SensorCalibration,
}

impl SPI for UserSensorCalibration {
    fn range() -> SPIRange {
        RANGE_USER_CALIBRATION_SENSORS
    }
}
impl From<SensorCalibration> for UserSensorCalibration {
    fn from(calib: SensorCalibration) -> Self {
        UserSensorCalibration {
            magic: USER_CALIB_MAGIC,
            calib,
        }
    }
}

impl From<UserSensorCalibration> for SPIWriteRequest {
    fn from(calib: UserSensorCalibration) -> Self {
        let range = UserSensorCalibration::range();
        SPIWriteRequest {
            address: range.0.to_le_bytes(),
            size: range.1,
            data: SPIData {
                imu_user_calib: calib.into(),
            },
        }
    }
}

impl TryFrom<SPIReadResult> for UserSensorCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.imu_user_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
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

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Color(u8, u8, u8);

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl FromStr for Color {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: do better
        assert!(s.len() == 6);
        Ok(Color(
            u8::from_str_radix(s.get(0..2).unwrap(), 16)?,
            u8::from_str_radix(s.get(2..4).unwrap(), 16)?,
            u8::from_str_radix(s.get(4..6).unwrap(), 16)?,
        ))
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ControllerColor {
    pub body: Color,
    pub buttons: Color,
    pub left_grip: Color,
    pub right_grip: Color,
}

impl SPI for ControllerColor {
    fn range() -> SPIRange {
        RANGE_CONTROLLER_COLOR
    }
}

impl TryFrom<SPIReadResult> for ControllerColor {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.color })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}
