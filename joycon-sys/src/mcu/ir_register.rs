use crate::common::*;
use num::ToPrimitive;

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct Register {
    address: U16LE,
    value: u8,
}

impl Register {
    fn new(address: Address, value: u8) -> Register {
        Register {
            address: address.into(),
            value,
        }
    }
    pub fn resolution(resolution: Resolution) -> Register {
        Register::new(Resolution, resolution as u8)
    }

    pub fn exposure(exposure: u16) -> [Register; 2] {
        [
            Register::new(ExposureLSB, (exposure & 0xff) as u8),
            Register::new(ExposureMSB, (exposure >> 8) as u8),
        ]
    }

    pub fn exposure_mode(mode: ExposureMode) -> Register {
        Register::new(ExposureMode, mode as u8)
    }

    pub fn digital_gain(gain: u16) -> [Register; 2] {
        [
            Register::new(DigitalGainLSB, ((gain & 0x0f) << 4) as u8),
            Register::new(DigitalGainMSB, ((gain & 0xf0) >> 4) as u8),
        ]
    }

    pub fn external_light_filter(filter: ExternalLightFilter) -> Register {
        Register::new(DigitalGainLSB, filter as u8)
    }

    pub fn white_pixel_threshold(threshold: u8) -> Register {
        Register::new(WhitePixelThreshold, threshold)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
enum Address {
    Resolution = 0x2e00,
    DigitalGainLSB = 0x2e01,
    DigitalGainMSB = 0x2f01,
    ExposureLSB = 0x3001,
    ExposureMSB = 0x3101,
    ExposureMode = 0x3201,
    ExternalLightFilter = 0x0e00,
    WhitePixelThreshold = 0x4301,
}
use Address::*;

impl From<Address> for U16LE {
    fn from(address: Address) -> U16LE {
        U16LE::from(address.to_u16().unwrap())
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum Resolution {
    /// Full pixel array
    R320x240 = 0b0000_0000,
    /// Sensor Binning [2 X 2]
    R160x120 = 0b0101_0000,
    /// Sensor Binning [4 x 2] and Skipping [1 x 2]
    R80x50 = 0b0110_0100,
    /// Sensor Binning [4 x 2] and Skipping [2 x 4]
    R40x30 = 0b0110_1001,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum ExposureMode {
    Manual = 0,
    Max = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum ExternalLightFilter {
    Off = 0b00,
    X1 = 0b11,
}
