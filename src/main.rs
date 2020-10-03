mod constants;
mod game;
#[allow(unused)]
mod gl;
mod graphics;
mod input;
mod mixer;
mod platform;
mod texture_atlas;

use constants::{SCREEN_SIZE, TICK_DT};
use game::Game;
use input::InputEvent;

fn main() {
    platform::run(
        "My game",
        (SCREEN_SIZE.width, SCREEN_SIZE.height),
        |gl_context: &mut gl::Context| {
            let mut game = Game::new(gl_context);
            let mut input_vec = Vec::new();
            let mut last_update: f32 = 0.;
            move |dt: f32, inputs: &[InputEvent], gl_context: &mut gl::Context| {
                // accumulate input over several frames
                input_vec.extend_from_slice(inputs);

                // jank ass fixed update loop
                last_update += dt;
                if last_update > TICK_DT {
                    game.update(&input_vec);

                    last_update -= TICK_DT;
                    input_vec.clear();
                }

                game.draw(gl_context);
            }
        },
    )
}