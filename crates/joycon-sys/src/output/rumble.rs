#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RumbleData {
    pub left: RumbleSide,
    pub right: RumbleSide,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(non_snake_case)]
pub struct RumbleSide {
    hb_freq_msB: u8,
    hb_freq_lsb_amp_high: u8,
    lb_freq_amp_low_msb: u8,
    amp_low_lsB: u8,
}

impl RumbleSide {
    pub fn from_freq(
        mut hi_freq: f32,
        mut hi_amp: f32,
        mut low_freq: f32,
        mut low_amp: f32,
    ) -> RumbleSide {
        hi_freq = hi_freq.max(82.).min(1253.);
        low_freq = low_freq.max(41.).min(626.);
        low_amp = low_amp.max(0.).min(1.);
        hi_amp = hi_amp.max(0.).min(1.);

        let hi_freq_hex = (Self::encode_freq(hi_freq) - 0x60) * 4;
        let low_freq_hex = (Self::encode_freq(low_freq) - 0x40) as u8;
        let hi_amp_hex = ((100. * hi_amp) as u8) << 1;
        let low_amp_hex = ((228. - 128.) * low_amp) as u8 + 0x80;
        RumbleSide::from_encoded(
            [hi_freq_hex as u8, (hi_freq_hex >> 8) as u8],
            hi_amp_hex,
            low_freq_hex,
            [(low_amp_hex & 1) << 7, low_amp_hex >> 1],
        )
    }

    fn encode_freq(f: f32) -> u16 {
        ((f / 10.).log2() * 32.).round() as u16
    }

    fn from_encoded(
        high_freq: [u8; 2],
        high_amp: u8,
        low_freq: u8,
        low_amp: [u8; 2],
    ) -> RumbleSide {
        assert_eq!(high_freq[0] & 0b11, 0);
        assert_eq!(high_freq[1] & 0xfe, 0);
        assert_eq!(high_amp & 1, 0);
        assert!(high_amp <= 0xc8);
        assert_eq!(low_freq & 0x80, 0);
        assert_eq!(low_amp[0] & 0x7f, 0);
        assert!(0x40 <= low_amp[1] && low_amp[1] <= 0x72);
        RumbleSide {
            hb_freq_msB: high_freq[0],
            hb_freq_lsb_amp_high: high_freq[1] | high_amp,
            lb_freq_amp_low_msb: low_freq | low_amp[0],
            amp_low_lsB: low_amp[1],
        }
    }
}

impl Default for RumbleSide {
    fn default() -> Self {
        RumbleSide::from_freq(320., 0., 160., 0.)
    }
}

#[test]
fn encode_rumble() {
    let rumble = RumbleSide::from_freq(320., 0., 160., 0.);
    assert_eq!(
        rumble,
        RumbleSide {
            hb_freq_msB: 0x00,
            hb_freq_lsb_amp_high: 0x01,
            lb_freq_amp_low_msb: 0x40,
            amp_low_lsB: 0x40,
        }
    );
}
