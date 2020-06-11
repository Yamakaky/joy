#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct PlayerLights(u8);

impl PlayerLights {
    #[allow(clippy::identity_op)]
    pub fn new(p0: PlayerLight, p1: PlayerLight, p2: PlayerLight, p3: PlayerLight) -> PlayerLights {
        use PlayerLight::*;
        PlayerLights(
            ((p0 == On) as u8) << 0
                | ((p1 == On) as u8) << 1
                | ((p2 == On) as u8) << 2
                | ((p3 == On) as u8) << 3
                | ((p0 == Blinking) as u8) << 4
                | ((p1 == Blinking) as u8) << 5
                | ((p2 == Blinking) as u8) << 6
                | ((p3 == Blinking) as u8) << 7,
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PlayerLight {
    Off,
    Blinking,
    On,
}

impl From<bool> for PlayerLight {
    fn from(b: bool) -> Self {
        if b {
            PlayerLight::On
        } else {
            PlayerLight::Off
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct HomeLight {
    s1: Settings1,
    s2: Settings2,
    cycles: [HomeLightCycle; 8],
}

impl HomeLight {
    pub fn new(
        mini_cycle_duration: u8,
        led_start_intensity: u8,
        nb_full_cycles: u8,
        led_cycles: &[(u8, u8, u8)],
    ) -> HomeLight {
        assert!(led_cycles.len() <= 15);
        assert!(mini_cycle_duration <= 0xf);
        assert!(led_start_intensity <= 0xf);
        assert!(nb_full_cycles <= 0xf);

        let mut s1 = Settings1(0);
        s1.set_nb_mini_cycles(led_cycles.len() as u8);
        s1.set_mini_cycle_duration(mini_cycle_duration);

        let mut s2 = Settings2(0);
        s2.set_led_start_intensity(led_start_intensity);
        s2.set_nb_full_cycles(nb_full_cycles);

        let mut cycles = [HomeLightCycle::default(); 8];
        let mut it = led_cycles.iter();
        let mut i = 0;
        while let (Some(c1), c2) = (it.next(), it.next()) {
            let entry = &mut cycles[i];
            assert!(c1.0 <= 0xf);
            assert!(c1.1 <= 0xf);
            assert!(c1.2 <= 0xf);

            entry.intensity.set_first(c1.0);
            entry.first_duration.set_fading_transition(c1.1);
            entry.first_duration.set_led_duration(c1.2);
            if let Some(c2) = c2 {
                assert!(c2.0 <= 0xf);
                assert!(c2.1 <= 0xf);
                assert!(c2.2 <= 0xf);
                entry.intensity.set_second(c2.0);
                entry.second_duration.set_fading_transition(c2.1);
                entry.second_duration.set_led_duration(c2.2);
            }
            i += 1;
        }

        HomeLight { s1, s2, cycles }
    }
}

bitfield::bitfield! {
    #[derive(Copy, Clone)]
    struct Settings1(u8);
    impl Debug;
    nb_mini_cycles, set_nb_mini_cycles: 7, 4;
    mini_cycle_duration, set_mini_cycle_duration: 3, 0;
}

bitfield::bitfield! {
    #[derive(Copy, Clone)]
    struct Settings2(u8);
    impl Debug;
    led_start_intensity, set_led_start_intensity: 7, 4;
    nb_full_cycles, set_nb_full_cycles: 3, 0;
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
struct HomeLightCycle {
    intensity: Intensity,
    first_duration: Durations,
    second_duration: Durations,
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    struct Intensity(u8);
    impl Debug;
    first, set_first: 7, 4;
    second, set_second: 3, 0;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    struct Durations(u8);
    impl Debug;
    fading_transition, set_fading_transition: 7, 4;
    led_duration, set_led_duration: 3, 0;
}

#[cfg(test)]
#[test]
fn check_layout() {
    assert_eq!(26, std::mem::size_of::<HomeLight>());
}
