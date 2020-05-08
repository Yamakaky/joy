use enigo::*;

#[derive(Copy, Clone, Debug, Default)]
pub struct Mouse {
    diff_x: f32,
    diff_y: f32,
}

impl Mouse {
    pub fn move_relative(&mut self, enigo: &mut Enigo, mut x: f32, mut y: f32) {
        // enigo works with pixels, so we keep the remainder to not smooth the small movements.
        x += self.diff_x;
        y += self.diff_y;
        let round_x = x.round();
        let round_y = y.round();
        self.diff_x = x - round_x;
        self.diff_y = y - round_y;

        enigo.mouse_move_relative(round_x as i32, round_y as i32);
    }
}
