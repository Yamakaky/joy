use cgmath::{vec2, vec3, Deg, Euler, InnerSpace, Quaternion, Rotation, Vector2, Vector3};
use hid_gamepad::sys::Motion;

pub fn map_input(
    motion: &Motion,
    dt: f64,
    sensor_fusion: &mut dyn SensorFusion,
    mapper: &mut dyn SpaceMapper,
) -> Vector2<f64> {
    let rot = vec3(
        motion.rotation_speed.x.0,
        -motion.rotation_speed.y.0,
        motion.rotation_speed.z.0,
    ) * dt;
    let rot2 = Euler::new(
        motion.rotation_speed.x * dt,
        -motion.rotation_speed.y * dt,
        motion.rotation_speed.z * dt,
    );
    let up_vector = sensor_fusion.compute_up_vector(rot2, motion.acceleration);
    mapper.map(rot, up_vector)
}
pub trait SensorFusion {
    fn compute_up_vector(&mut self, rot: Euler<Deg<f64>>, acc: Vector3<f64>) -> Vector3<f64>;
}

/// Convert local space motion to 2D mouse-like motion.
pub trait SpaceMapper {
    fn map(&self, rot: Vector3<f64>, grav: Vector3<f64>) -> Vector2<f64>;
}

#[derive(Debug, Copy, Clone)]
pub struct SimpleFusion {
    up_vector: Vector3<f64>,
    correction_factor: f64,
}

impl SimpleFusion {
    pub fn new() -> Self {
        Self {
            up_vector: vec3(0., -1., 0.),
            correction_factor: 0.02,
        }
    }
}

impl SensorFusion for SimpleFusion {
    fn compute_up_vector(&mut self, rot: Euler<Deg<f64>>, acc: Vector3<f64>) -> Vector3<f64> {
        let rotation = Quaternion::from(rot).invert();
        self.up_vector = rotation.rotate_vector(self.up_vector);
        self.up_vector -= (acc + self.up_vector) * self.correction_factor;
        self.up_vector
    }
}


#[derive(Default)]
pub struct LocalSpace;

impl SpaceMapper for LocalSpace {
    fn map(&self, rot: Vector3<f64>, _up_vector: Vector3<f64>) -> Vector2<f64> {
        vec2(-rot.y, rot.x)
    }
}

#[derive(Default)]
pub struct WorldSpace;

impl SpaceMapper for WorldSpace {
    fn map(&self, rot: Vector3<f64>, up_vector: Vector3<f64>) -> Vector2<f64> {
        let flatness = up_vector.y.abs();
        let upness = up_vector.z.abs();
        let side_reduction = (flatness.max(upness) - 0.125).clamp(0., 1.);

        let yaw_diff = -rot.dot(up_vector);

        let pitch = vec3(1., 0., 0.) - up_vector * up_vector.x;
        let pitch_diff = if pitch.magnitude2() != 0. {
            side_reduction * rot.dot(pitch.normalize())
        } else {
            0.
        };
        vec2(yaw_diff, pitch_diff)
    }
}

pub struct PlayerSpace {
    yaw_relax_factor: f64,
}

impl Default for PlayerSpace {
    fn default() -> Self {
        Self {
            yaw_relax_factor: 1.41,
        }
    }
}

impl SpaceMapper for PlayerSpace {
    fn map(&self, rot: Vector3<f64>, up_vector: Vector3<f64>) -> Vector2<f64> {
        let world_yaw = rot.y * up_vector.y + rot.z * up_vector.z;
        vec2(
            -world_yaw.signum()
                * (world_yaw.abs() * self.yaw_relax_factor).min(vec2(rot.y, rot.z).magnitude()),
            rot.x,
        )
    }
}
