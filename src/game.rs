use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use euclid::{
    default::{Point2D, Transform2D, Vector2D},
    point2, vec2,
};

use crate::{
    constants::{SCREEN_SIZE, TICK_DT, ZOOM_LEVEL},
    gl,
    graphics::{load_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key},
    level::{create_level, generate_tile_buffer, DoorTile, Level, Tile, TILE_SIZE},
    mixer::{Audio, Mixer},
    texture_atlas::{TextureAtlas, TextureRect},
};

pub struct Game {
    program: gl::Program,
    ground_buffer: gl::VertexBuffer,
    vertex_buffer: gl::VertexBuffer,
    assets: Assets,

    tick: usize,
    rewind: bool,
    clear_players: bool,
    paused: bool,

    ui: Sprite,
    ui_bulb: Sprite,
    ui_time_bar_bg: Sprite,
    ui_time_bar: Sprite,
    ui_win_screen: Sprite,
    ui_vertex_buffer: gl::VertexBuffer,

    level: Level,
    controls: Controls,

    players: Vec<Ghost>,
    buttons: HashMap<Point2D<i32>, Button>,
    doors: HashMap<Point2D<i32>, Door>,
    teleporters: HashMap<Point2D<i32>, Teleporter>,
    bulbs: Vec<Bulb>,
    the_machine: TheMachine,

    mixer: Arc<Mixer>,
}

impl Game {
    pub fn new(gl_context: &mut gl::Context, mixer: Arc<Mixer>) -> Self {
        let vertex_shader = unsafe {
            gl_context
                .create_shader(gl::ShaderType::Vertex, include_str!("shaders/shader.vert"))
                .unwrap()
        };
        let fragment_shader = unsafe {
            gl_context
                .create_shader(
                    gl::ShaderType::Fragment,
                    include_str!("shaders/shader.frag"),
                )
                .unwrap()
        };

        let mut program = unsafe {
            gl_context
                .create_program(&gl::ProgramDescriptor {
                    vertex_shader: &vertex_shader,
                    fragment_shader: &fragment_shader,
                    uniforms: &[
                        gl::UniformEntry {
                            name: "u_transform",
                            ty: gl::UniformType::Mat3,
                        },
                        gl::UniformEntry {
                            name: "u_texture",
                            ty: gl::UniformType::Texture,
                        },
                    ],
                    vertex_format: gl::VertexFormat {
                        stride: std::mem::size_of::<Vertex>(),
                        attributes: &[
                            gl::VertexAttribute {
                                name: "a_pos",
                                ty: gl::VertexAttributeType::Float,
                                size: 2,
                                offset: 0,
                            },
                            gl::VertexAttribute {
                                name: "a_uv",
                                ty: gl::VertexAttributeType::Float,
                                size: 2,
                                offset: 2 * 4,
                            },
                            gl::VertexAttribute {
                                name: "a_color",
                                ty: gl::VertexAttributeType::Float,
                                size: 4,
                                offset: 4 * 4,
                            },
                        ],
                    },
                })
                .unwrap()
        };

        let mut texture = unsafe {
            gl_context
                .create_texture(
                    gl::TextureFormat::RGBAFloat,
                    TEXTURE_ATLAS_SIZE.width,
                    TEXTURE_ATLAS_SIZE.height,
                )
                .unwrap()
        };
        let mut atlas = TextureAtlas::new((TEXTURE_ATLAS_SIZE.width, TEXTURE_ATLAS_SIZE.height));
        program
            .set_uniform(1, gl::Uniform::Texture(&texture))
            .unwrap();

        let vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };

        let assets = unsafe {
            Assets {
                ghost: load_image(
                    include_bytes!("../assets/player.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ghost_shadow: load_image(
                    include_bytes!("../assets/ghost_shadow.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ground: load_image(
                    include_bytes!("../assets/ground.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                walls: load_image(
                    include_bytes!("../assets/walls.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                door_h: load_image(
                    include_bytes!("../assets/door.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                door_v: load_image(
                    include_bytes!("../assets/door_v.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                button: load_image(
                    include_bytes!("../assets/button.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                teleporter: load_image(
                    include_bytes!("../assets/teleporter.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                bulb: load_image(
                    include_bytes!("../assets/bulb.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                bulb_shadow: load_image(
                    include_bytes!("../assets/bulb_shadow.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                the_machine: load_image(
                    include_bytes!("../assets/the_machine.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                the_machine_slots: load_image(
                    include_bytes!("../assets/the_machine_slots.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ui: load_image(include_bytes!("../assets/ui.png"), &mut atlas, &mut texture)
                    .unwrap(),
                ui_bulb: load_image(
                    include_bytes!("../assets/ui_bulb.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ui_time_bar: load_image(
                    include_bytes!("../assets/ui_time_bar.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                ui_time_bar_bg: load_image(
                    include_bytes!("../assets/ui_time_bar_bg.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),
                win_screen: load_image(
                    include_bytes!("../assets/win_screen.png"),
                    &mut atlas,
                    &mut texture,
                )
                .unwrap(),

                door_sound: mixer
                    .load_ogg(include_bytes!("../assets/door.ogg"))
                    .unwrap(),
                drop_sound: mixer
                    .load_ogg(include_bytes!("../assets/drop.ogg"))
                    .unwrap(),
                pickup_sound: mixer
                    .load_ogg(include_bytes!("../assets/pickup.ogg"))
                    .unwrap(),
                rewind_sound: mixer
                    .load_ogg(include_bytes!("../assets/rewind.ogg"))
                    .unwrap(),
                start_sound: mixer
                    .load_ogg(include_bytes!("../assets/start.ogg"))
                    .unwrap(),
                teleport_sound: mixer
                    .load_ogg(include_bytes!("../assets/teleport.ogg"))
                    .unwrap(),
            }
        };

        let level = create_level();
        let ground_buffer = generate_tile_buffer(&level, assets.ground, assets.walls, gl_context);

        let mut buttons = HashMap::new();
        for (position, button_tile) in level.buttons.iter() {
            buttons.insert(
                *position,
                Button::new(assets.button, *position, button_tile.connections.clone()),
            );
        }

        let mut doors = HashMap::new();
        for (position, door_tile) in level.doors.iter() {
            doors.insert(
                *position,
                Door::new(
                    match door_tile {
                        &DoorTile::Horizontal => assets.door_h,
                        &DoorTile::Vertical => assets.door_v,
                    },
                    *position,
                ),
            );
        }

        let mut teleporters = HashMap::new();
        for (position, teleporter_tile) in level.teleporters.iter() {
            teleporters.insert(
                *position,
                Teleporter::new(
                    assets.teleporter,
                    *position,
                    teleporter_tile.connection.expect("unconnected teleporter"),
                ),
            );
        }

        let mut bulbs = Vec::new();
        for position in level.bulbs.iter() {
            bulbs.push(Bulb::new(
                assets.bulb,
                assets.bulb_shadow,
                position.to_f32() + vec2(0.5, 0.5),
            ));
        }

        let players = vec![Ghost::new(
            assets.ghost,
            assets.ghost_shadow,
            level.player_start,
        )];

        let the_machine = TheMachine::new(
            assets.the_machine,
            assets.the_machine_slots,
            assets.bulb,
            level.the_machine.to_f32(),
        );

        let ui = Sprite::new(assets.ui, 1, point2(0., 0.));
        let ui_bulb = Sprite::new(assets.ui_bulb, 1, point2(0., 0.));
        let ui_time_bar = Sprite::new(assets.ui_time_bar, 1, point2(0., 0.));
        let ui_time_bar_bg = Sprite::new(assets.ui_time_bar_bg, 1, point2(0., 0.));
        let ui_win_screen = Sprite::new(assets.win_screen, 1, point2(0., 0.));

        let ui_vertex_buffer = unsafe { gl_context.create_vertex_buffer() }.unwrap();

        Self {
            program,
            ground_buffer,
            vertex_buffer,
            assets,

            tick: 0,
            rewind: false,
            clear_players: false,
            paused: true,

            ui,
            ui_bulb,
            ui_time_bar,
            ui_time_bar_bg,
            ui_win_screen,
            ui_vertex_buffer,

            level,
            controls: Controls::default(),

            players,
            buttons,
            doors,
            teleporters,
            bulbs,
            the_machine,

            mixer,
        }
    }

    pub fn update(&mut self, inputs: &[InputEvent]) {
        if self.the_machine.slots_occupied == 6 && self.tick == 0 {
            return;
        }

        for input in inputs {
            match input {
                InputEvent::KeyDown(Key::W) => {
                    self.controls.up = true;
                }
                InputEvent::KeyUp(Key::W) => {
                    self.controls.up = false;
                }
                InputEvent::KeyDown(Key::A) => {
                    self.controls.left = true;
                }
                InputEvent::KeyUp(Key::A) => {
                    self.controls.left = false;
                }
                InputEvent::KeyDown(Key::S) => {
                    self.controls.down = true;
                }
                InputEvent::KeyUp(Key::S) => {
                    self.controls.down = false;
                }
                InputEvent::KeyDown(Key::D) => {
                    self.controls.right = true;
                }
                InputEvent::KeyUp(Key::D) => {
                    self.controls.right = false;
                }
                InputEvent::KeyDown(Key::Escape) => {
                    self.rewind = false;
                    self.players = vec![Ghost::new(
                        self.assets.ghost,
                        self.assets.ghost_shadow,
                        self.level.player_start,
                    )];
                    self.tick = 0;
                }
                InputEvent::KeyDown(Key::Space) => {
                    if !self.rewind {
                        self.mixer.play(&self.assets.rewind_sound, 0.125, false);
                    }
                    self.rewind = true;
                    self.paused = true;
                }
                InputEvent::KeyDown(Key::R) => {
                    if !self.rewind {
                        self.mixer.play(&self.assets.rewind_sound, 0.125, false);
                    }
                    self.rewind = true;
                    self.paused = true;
                    self.clear_players = true;
                }
                _ => {}
            }
        }

        // only current player gets new inputs
        if self.rewind {
            self.tick = self.tick.saturating_sub(5);

            if self.tick == 0 {
                self.rewind = false;

                if self.clear_players {
                    self.players = vec![Ghost::new(
                        self.assets.ghost,
                        self.assets.ghost_shadow,
                        self.level.player_start,
                    )];

                    self.clear_players = false;
                } else {
                    for player in self.players.iter_mut() {
                        player.reset(self.level.player_start);
                    }
                    self.players
                        .last_mut()
                        .unwrap()
                        .set_color([1.0, 1.0, 1.0, 0.5]);
                    self.players.push(Ghost::new(
                        self.assets.ghost,
                        self.assets.ghost_shadow,
                        self.level.player_start,
                    ));
                }

                for bulb in self.bulbs.iter_mut() {
                    bulb.reset();
                }
            }
        } else {
            if self.paused {
                if self.controls.down
                    || self.controls.up
                    || self.controls.left
                    || self.controls.right
                {
                    self.paused = false;
                    self.mixer.play(&self.assets.start_sound, 0.125, false);
                }
            }
            if !self.paused {
                self.players
                    .last_mut()
                    .unwrap()
                    .push_controls(self.controls);

                // all players are updated
                for player in self.players.iter_mut() {
                    player.update(self.tick, &self.level, &self.doors);
                }

                self.tick += 1;
                if self.tick >= LOOP_TICKS {
                    if !self.rewind {
                        self.mixer.play(&self.assets.rewind_sound, 0.125, false);
                    }
                    self.rewind = true;
                    self.paused = true;
                }
            }
        }

        let mut players_spatial: HashMap<Point2D<i32>, Vec<usize>> = HashMap::new();
        for (index, player) in self.players.iter().enumerate() {
            players_spatial
                .entry(point2(
                    player.position(self.tick).x.floor() as i32,
                    player.position(self.tick).y.floor() as i32,
                ))
                .or_insert(Vec::new())
                .push(index);
        }
        for button in self.buttons.values_mut() {
            button.update(
                &players_spatial,
                &mut self.players,
                &mut self.doors,
                &mut self.teleporters,
                &self.mixer,
                &self.assets,
            );
        }
        for teleporter in self.teleporters.values_mut() {
            teleporter.update();
        }

        for bulb in self.bulbs.iter_mut() {
            bulb.update(
                self.tick,
                self.paused,
                &players_spatial,
                &self.players,
                &self.the_machine,
                &self.mixer,
                &self.assets,
            );
            if bulb.inserted {
                self.the_machine.add_bulb();
                self.rewind = true;
                self.mixer.play(&self.assets.drop_sound, 0.125, false);
                self.paused = true;
                self.clear_players = true;
            }
        }
        self.bulbs.retain(|bulb| !bulb.inserted);

        self.the_machine.update();
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        let mut vertices = Vec::new();

        for button in self.buttons.values() {
            button.draw(&mut vertices);
        }
        for door in self.doors.values() {
            door.draw(&mut vertices);
        }
        for teleporter in self.teleporters.values() {
            teleporter.draw(&mut vertices);
        }
        self.the_machine.draw(&mut vertices);

        // draw all shadows first
        for player in self.players.iter() {
            player.draw_shadow(self.tick, &mut vertices);
        }

        // then players
        for player in self.players.iter() {
            player.draw(self.tick, &mut vertices);
        }

        for bulb in self.bulbs.iter() {
            bulb.draw(self.tick, &mut vertices);
        }

        // ui stuff
        let mut ui_vertices = Vec::new();

        if self.the_machine.slots_occupied == 6 && self.tick == 0 {
            render_sprite(&self.ui_win_screen, 0, point2(0., 0.), &mut ui_vertices);
        } else {
            render_sprite(&self.ui_time_bar_bg, 0, point2(244., 67.), &mut ui_vertices);

            self.ui_time_bar
                .set_transform(Transform2D::create_translation(
                    0.,
                    self.tick as f32 / LOOP_TICKS as f32 * -80.,
                ));
            render_sprite(&self.ui_time_bar, 0, point2(244., 67.), &mut ui_vertices);

            render_sprite(&self.ui, 0, point2(0., 0.), &mut ui_vertices);
            if self.the_machine.slots_occupied >= 1 {
                render_sprite(&self.ui_bulb, 0, point2(207., 176.), &mut ui_vertices);
            }
            if self.the_machine.slots_occupied >= 2 {
                render_sprite(&self.ui_bulb, 0, point2(207. + 20., 176.), &mut ui_vertices);
            }
            if self.the_machine.slots_occupied >= 3 {
                render_sprite(&self.ui_bulb, 0, point2(207. + 39., 176.), &mut ui_vertices);
            }
            if self.the_machine.slots_occupied >= 4 {
                render_sprite(&self.ui_bulb, 0, point2(207., 176. - 22.), &mut ui_vertices);
            }
            if self.the_machine.slots_occupied >= 5 {
                render_sprite(
                    &self.ui_bulb,
                    0,
                    point2(207. + 20., 176. - 22.),
                    &mut ui_vertices,
                );
            }
            if self.the_machine.slots_occupied >= 6 {
                render_sprite(
                    &self.ui_bulb,
                    0,
                    point2(207. + 39., 176. - 22.),
                    &mut ui_vertices,
                );
            }
        }

        unsafe {
            self.vertex_buffer.write(&vertices);
            self.ui_vertex_buffer.write(&ui_vertices);

            context.clear([75. / 255., 58. / 255., 58. / 255., 1.]);

            let mut camera_pos = self.players.last().unwrap().position(self.tick);
            // Fixes tile gaps but causes stuttery camera RIP
            camera_pos.x = (camera_pos.x * ZOOM_LEVEL * TILE_SIZE as f32).floor()
                / ZOOM_LEVEL
                / TILE_SIZE as f32;
            camera_pos.y = (camera_pos.y * ZOOM_LEVEL * TILE_SIZE as f32).floor()
                / ZOOM_LEVEL
                / TILE_SIZE as f32;
            let transform = Transform2D::create_translation(-camera_pos.x - 3., -camera_pos.y - 0.)
                .post_scale(
                    1.0 / SCREEN_SIZE.width as f32,
                    1.0 / SCREEN_SIZE.height as f32,
                )
                .post_scale(ZOOM_LEVEL, ZOOM_LEVEL)
                .post_scale(TILE_SIZE as f32, TILE_SIZE as f32)
                .post_scale(2., 2.);

            self.program
                .set_uniform(
                    0,
                    gl::Uniform::Mat3([
                        [transform.m11, transform.m12, 0.0],
                        [transform.m21, transform.m22, 0.0],
                        [transform.m31, transform.m32, 1.0],
                    ]),
                )
                .unwrap();

            self.program.render_vertices(&self.ground_buffer).unwrap();
            self.program.render_vertices(&self.vertex_buffer).unwrap();

            let ui_transform = Transform2D::create_scale(
                1.0 / SCREEN_SIZE.width as f32,
                1.0 / SCREEN_SIZE.height as f32,
            )
            .post_scale(3., 3.)
            .post_scale(2., 2.)
            .post_translate(vec2(-1., -1.));
            self.program
                .set_uniform(
                    0,
                    gl::Uniform::Mat3([
                        [ui_transform.m11, ui_transform.m12, 0.0],
                        [ui_transform.m21, ui_transform.m22, 0.0],
                        [ui_transform.m31, ui_transform.m32, 1.0],
                    ]),
                )
                .unwrap();
            self.program
                .render_vertices(&self.ui_vertex_buffer)
                .unwrap();
        }
    }
}

struct Assets {
    ghost: TextureRect,
    ghost_shadow: TextureRect,
    ground: TextureRect,
    walls: TextureRect,
    door_h: TextureRect,
    door_v: TextureRect,
    button: TextureRect,
    teleporter: TextureRect,
    bulb: TextureRect,
    bulb_shadow: TextureRect,
    the_machine: TextureRect,
    the_machine_slots: TextureRect,
    ui: TextureRect,
    ui_bulb: TextureRect,
    ui_time_bar: TextureRect,
    ui_time_bar_bg: TextureRect,
    win_screen: TextureRect,

    door_sound: Audio,
    drop_sound: Audio,
    pickup_sound: Audio,
    rewind_sound: Audio,
    start_sound: Audio,
    teleport_sound: Audio,
}

#[derive(Default, Clone, Copy)]
struct Controls {
    up: bool,
    left: bool,
    down: bool,
    right: bool,
}

struct Ghost {
    sprite: Sprite,
    shadow: Sprite,
    controls: Vec<Controls>,
    positions: Vec<Point2D<f32>>,
    animation_timer: f32,
}

impl Ghost {
    pub fn new(image: TextureRect, shadow: TextureRect, position: Point2D<f32>) -> Self {
        let mut sprite = Sprite::new(image, GHOST_ANIMATION_FRAMES, point2(6., -4.0));
        let mut shadow = Sprite::new(shadow, 1, point2(6., 3.));

        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        shadow.set_transform(transform);

        Self {
            sprite,
            shadow,
            controls: Vec::new(),
            positions: vec![position],
            animation_timer: 0.,
        }
    }

    pub fn teleport(&mut self, destination: Point2D<f32>) {
        *self.positions.last_mut().unwrap() = destination;
    }

    pub fn reset(&mut self, position: Point2D<f32>) {
        self.positions = vec![position];
        self.animation_timer = 0.;
    }

    pub fn push_controls(&mut self, controls: Controls) {
        self.controls.push(controls);
    }

    pub fn position(&self, tick: usize) -> Point2D<f32> {
        *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"))
    }

    pub fn update(&mut self, tick: usize, level: &Level, doors: &HashMap<Point2D<i32>, Door>) {
        if let Some(controls) = self.controls.get(tick) {
            let mut dir: Vector2D<f32> = vec2(0., 0.);
            if controls.up {
                dir.y += 1.;
            }
            if controls.down {
                dir.y -= 1.;
            }
            if controls.right {
                dir.x += 1.;
            }
            if controls.left {
                dir.x -= 1.;
            }

            if dir.length() > 0. {
                // This is the laziest collision detection and resolution in the history of video gam
                let new_pos = *self.positions.last().expect("position vec is empty")
                    + dir.normalize() * GHOST_SPEED * TICK_DT;

                let mut colliding = false;
                let new_pos_tile = point2(new_pos.x.floor() as i32, new_pos.y.floor() as i32);
                if level.tile(new_pos_tile.x, new_pos_tile.y) == Tile::Wall {
                    colliding = true;
                }
                if doors
                    .get(&new_pos_tile)
                    .map(|door| !door.is_open())
                    .unwrap_or(false)
                {
                    colliding = true;
                }

                if !colliding {
                    self.positions.push(new_pos);
                } else {
                    self.positions
                        .push(*self.positions.last().expect("position vec is empty"));
                }
            } else {
                self.positions
                    .push(*self.positions.last().expect("position vec is empty"));
            }
        }

        self.animation_timer = (self.animation_timer + TICK_DT) % GHOST_ANIMATION_TIME;
    }

    pub fn draw_shadow(&self, tick: usize, out: &mut Vec<Vertex>) {
        let position = *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"));
        render_sprite(&self.shadow, 0, position, out);
    }

    pub fn draw(&self, tick: usize, out: &mut Vec<Vertex>) {
        let frame = (self.animation_timer / GHOST_ANIMATION_TIME * GHOST_ANIMATION_FRAMES as f32)
            .floor() as usize;
        let mut position = *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"));

        // fixes stuttery player caused by doing this same thing to the camera
        // give me my programming diploma please
        position.x =
            (position.x * ZOOM_LEVEL * TILE_SIZE as f32).floor() / ZOOM_LEVEL / TILE_SIZE as f32;
        position.y =
            (position.y * ZOOM_LEVEL * TILE_SIZE as f32).floor() / ZOOM_LEVEL / TILE_SIZE as f32;
        render_sprite(&self.sprite, frame, position, out);
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.sprite.set_color(color);
    }
}

struct Button {
    sprite: Sprite,
    position: Point2D<i32>,
    connections: Vec<Point2D<i32>>,
    active: bool,
}

impl Button {
    pub fn new(image: TextureRect, position: Point2D<i32>, connections: Vec<Point2D<i32>>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(0., 0.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        Self {
            sprite,
            position,
            connections,
            active: false,
        }
    }

    pub fn update(
        &mut self,
        players_spatial: &HashMap<Point2D<i32>, Vec<usize>>,
        players: &mut Vec<Ghost>,
        doors: &mut HashMap<Point2D<i32>, Door>,
        teleporters: &mut HashMap<Point2D<i32>, Teleporter>,
        mixer: &Mixer,
        assets: &Assets,
    ) {
        let mut changed_door = false;
        if players_spatial.contains_key(&self.position) {
            for connection in &self.connections {
                if let Some(door) = doors.get_mut(connection) {
                    if !door.open {
                        changed_door = true;
                    }
                    door.open = true;
                }

                // teleporters are edge triggered only
                if !self.active {
                    if let Some(teleporter) = teleporters.get_mut(connection) {
                        if teleporter.active_timer <= 0. {
                            mixer.play(&assets.teleport_sound, 0.125, false);
                        }
                        teleporter.activate(players_spatial, players);
                    }
                }
            }
            self.active = true;
        } else {
            self.active = false;
            for connection in &self.connections {
                if let Some(door) = doors.get_mut(connection) {
                    if door.open {
                        changed_door = true;
                    }
                    door.open = false;
                }
            }
        }

        if changed_door {
            mixer.play(&assets.door_sound, 0.125, false)
        }
    }

    pub fn draw(&self, out: &mut Vec<Vertex>) {
        render_sprite(
            &self.sprite,
            if self.active { 1 } else { 0 },
            self.position.to_f32(),
            out,
        );
    }
}

struct Door {
    sprite: Sprite,
    position: Point2D<i32>,
    open: bool,
}

impl Door {
    pub fn new(image: TextureRect, position: Point2D<i32>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(0., 0.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        Self {
            sprite,
            position,
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn draw(&self, out: &mut Vec<Vertex>) {
        render_sprite(
            &self.sprite,
            if self.open { 1 } else { 0 },
            self.position.to_f32(),
            out,
        );
    }
}

struct Teleporter {
    sprite: Sprite,
    position: Point2D<i32>,
    destination: Point2D<i32>,
    active_timer: f32,
}

impl Teleporter {
    pub fn new(image: TextureRect, position: Point2D<i32>, destination: Point2D<i32>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(0., 0.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        Self {
            sprite,
            position,
            destination,
            active_timer: 0.,
        }
    }

    pub fn update(&mut self) {
        self.active_timer = (self.active_timer - TICK_DT).max(0.);
    }

    pub fn activate(
        &mut self,
        players_spatial: &HashMap<Point2D<i32>, Vec<usize>>,
        players: &mut Vec<Ghost>,
    ) {
        if self.active_timer <= 0. {
            self.active_timer = 0.5;
            if let Some(teleport_entities) = players_spatial.get(&self.position) {
                for i in teleport_entities {
                    players[*i].teleport(self.destination.to_f32() + vec2(0.5, 0.5));
                }
            }
        }
    }

    pub fn draw(&self, out: &mut Vec<Vertex>) {
        render_sprite(
            &self.sprite,
            if self.active_timer > 0. { 1 } else { 0 },
            self.position.to_f32(),
            out,
        );
    }
}

struct Bulb {
    sprite: Sprite,
    shadow: Sprite,
    positions: Vec<Point2D<f32>>,
    bob_timer: f32,
    picked_up: Option<(usize, usize)>,
    inserted: bool,
}

impl Bulb {
    pub fn new(image: TextureRect, shadow: TextureRect, position: Point2D<f32>) -> Self {
        let mut sprite = Sprite::new(image, 2, point2(4., -2.));
        let mut shadow = Sprite::new(shadow, 1, point2(2., 1.5));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        shadow.set_transform(transform);
        Self {
            sprite,
            shadow,
            positions: vec![position],
            bob_timer: 0.,
            picked_up: None,
            inserted: false,
        }
    }

    pub fn position(&self, tick: usize) -> Point2D<f32> {
        *self
            .positions
            .get(tick + 1)
            .unwrap_or(self.positions.last().expect("positions vec is empty"))
    }

    pub fn update(
        &mut self,
        tick: usize,
        paused: bool,
        players_spatial: &HashMap<Point2D<i32>, Vec<usize>>,
        players: &Vec<Ghost>,
        the_machine: &TheMachine,
        mixer: &Mixer,
        assets: &Assets,
    ) {
        let picked_up = self.picked_up.map(|(t, _)| t <= tick).unwrap_or(false);
        if !picked_up {
            self.bob_timer = (self.bob_timer + TICK_DT) % 1.0;
            let height = ((self.bob_timer * 6.28).sin() + 1.) * 2.;
            let transform = Transform2D::create_translation(0., height)
                .post_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
            self.sprite.set_transform(transform);
        }

        if paused {
            return;
        }

        if let Some((_, pickup_player)) = self.picked_up {
            self.positions.push(players[pickup_player].position(tick));

            let transform = Transform2D::create_translation(1., 5.)
                .post_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
            self.sprite.set_transform(transform);

            if (the_machine.position - self.position(tick)).length() < 1. {
                self.inserted = true;
            }
        } else {
            self.positions.push(self.position(tick));

            let tile_pos = point2(
                self.position(tick).x.floor() as i32,
                self.position(tick).y.floor() as i32,
            );
            let mut near_players = Vec::new();
            for x in -1..1 {
                for y in -1..1 {
                    if let Some(ids) = players_spatial.get(&(tile_pos + vec2(x, y))) {
                        near_players.extend_from_slice(ids);
                    }
                }
            }

            near_players.sort_by(|a, b| {
                (self.position(tick) - players[*a].position(tick))
                    .length()
                    .partial_cmp(&(self.position(tick) - players[*b].position(tick)).length())
                    .unwrap_or(std::cmp::Ordering::Less)
            });
            if let Some(pickup_player) = near_players.first() {
                if (self.position(tick) - players[*pickup_player].position(tick)).length() < 0.5 {
                    self.picked_up = Some((tick, *pickup_player));
                    mixer.play(&assets.pickup_sound, 0.125, false);
                }
            }
        }
    }

    pub fn reset(&mut self) {
        let pos = *self.positions.first().unwrap();
        self.positions = vec![pos];
        self.picked_up = None;
    }

    pub fn draw(&self, tick: usize, out: &mut Vec<Vertex>) {
        let picked_up = self.picked_up.map(|(t, _)| t <= tick).unwrap_or(false);
        if !picked_up {
            render_sprite(&self.shadow, 0, self.position(tick).to_f32(), out);
        }
        render_sprite(
            &self.sprite,
            if picked_up { 1 } else { 0 },
            self.position(tick).to_f32(),
            out,
        );
    }
}

struct TheMachine {
    sprite: Sprite,
    slots: Sprite,
    bulb: Sprite,
    animation_timer: f32,
    position: Point2D<f32>,
    slots_occupied: usize,
}

impl TheMachine {
    pub fn new(
        image: TextureRect,
        slots: TextureRect,
        bulb: TextureRect,
        position: Point2D<f32>,
    ) -> Self {
        let mut sprite = Sprite::new(image, 3, point2(15., 0.));
        let mut slots = Sprite::new(slots, 6, point2(15., -17.));
        let mut bulb = Sprite::new(bulb, 2, point2(16., -18.));
        let transform = Transform2D::create_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32);
        sprite.set_transform(transform);
        slots.set_transform(transform);
        bulb.set_transform(transform);
        Self {
            sprite,
            slots,
            bulb,
            animation_timer: 0.,
            position,
            slots_occupied: 0,
        }
    }

    pub fn add_bulb(&mut self) {
        self.slots_occupied += 1;
    }

    pub fn update(&mut self) {
        self.animation_timer = (self.animation_timer + TICK_DT) % 0.25;
    }

    pub fn draw(&mut self, out: &mut Vec<Vertex>) {
        let frame = ((self.animation_timer / 0.25) * 3.).floor() as usize;
        render_sprite(&self.sprite, frame, self.position.to_f32(), out);

        if self.slots_occupied > 0 {
            render_sprite(
                &self.slots,
                self.slots_occupied - 1,
                self.position.to_f32(),
                out,
            );
            for i in 0..self.slots_occupied {
                self.bulb.set_transform(
                    Transform2D::create_translation(5. * i as f32, 0.)
                        .post_scale(1. / TILE_SIZE as f32, 1. / TILE_SIZE as f32),
                );
                render_sprite(&self.bulb, 1, self.position.to_f32(), out);
            }
        }
    }
}

// Time loops over 720 ticks, 12 seconds
const LOOP_TICKS: usize = 720;

const GHOST_SPEED: f32 = 5.;

const GHOST_ANIMATION_FRAMES: u32 = 6;
const GHOST_ANIMATION_TIME: f32 = 0.5;
