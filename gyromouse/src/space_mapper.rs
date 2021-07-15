use cgmath::{vec2, vec3, Deg, Euler, InnerSpace, Quaternion, Rotation, Vector2, Vector3, Zero};
use hid_gamepad::sys::Motion;

/// Convert local space motion to 2D mouse-like motion.
pub trait SpaceMapper {
    fn map(&self, rot: Vector3<f64>, grav: Vector3<f64>) -> Vector2<f64>;
    fn map_input(
        &self,
        motion: &Motion,
        gravity: &mut SensorFusionGravity,
        dt: f64,
    ) -> Vector2<f64> {
        let rot = vec3(
            motion.rotation_speed.y.0,
            motion.rotation_speed.z.0,
            motion.rotation_speed.x.0,
        ) * dt;
        gravity.update(rot, motion.acceleration);
        self.map(rot, gravity.gravity)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SensorFusionGravity {
    gravity: Vector3<f64>,
}

impl SensorFusionGravity {
    pub fn new() -> Self {
        Self {
            gravity: Vector3::zero(),
        }
    }
    fn update(&mut self, rot: Vector3<f64>, acc: Vector3<f64>) {
        let rotation =
            Quaternion::new((rot.magnitude() / 2.).cos(), -rot.x, -rot.y, -rot.z).normalize();
        self.gravity = rotation.rotate_vector(self.gravity);
        self.gravity -= (acc + self.gravity) * 0.02;
    }
}

pub struct LocalSpace;

impl SpaceMapper for LocalSpace {
    fn map(&self, rot: Vector3<f64>, _grav: Vector3<f64>) -> Vector2<f64> {
        vec2(rot.x, rot.y)
    }
}

pub struct WorldSpace;

impl SpaceMapper for WorldSpace {
    fn map(&self, rot: Vector3<f64>, grav: Vector3<f64>) -> Vector2<f64> {
        let flatness = grav.y.abs();
        let upness = grav.z.abs();
        let side_reduction = (flatness.max(upness) - 0.125).clamp(0., 1.);

        let yaw_diff = -rot.dot(grav);

        let pitch = vec3(1., 0., 0.) - grav * grav.x;
        let pitch_diff = if pitch.magnitude2() != 0. {
            side_reduction * rot.dot(pitch.normalize())
        } else {
            0.
        };
        vec2(pitch_diff, yaw_diff)
    }
}

pub struct PlayerSpace;

impl SpaceMapper for PlayerSpace {
    fn map(&self, rot: Vector3<f64>, grav: Vector3<f64>) -> Vector2<f64> {
        let world_yaw = rot.y * grav.y + rot.z * grav.z;
        let yaw_relax_factor = 1.41;
        vec2(
            rot.x,
            -world_yaw.signum()
                * (world_yaw.abs() * yaw_relax_factor).min(vec2(rot.y, rot.z).magnitude()),
        )
    }
}
