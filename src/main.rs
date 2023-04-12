mod components;
mod fly_camera;
mod fps_system;
mod modify_grid;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use components::{Cell, Simulation, WaterData, GRID_SIZE_HEIGHT, GRID_SIZE_WIDTH};
use fly_camera::{camera_2d_movement_system, FlyCamera2d};
use fps_system::DebugUiBundle;
use line_drawing::Supercover;
use modify_grid::modify_grid_system;
use ndarray::Array2;
use rand::seq::SliceRandom;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugUiBundle)
        .add_startup_system(add_grid_startup)
        .add_system(camera_2d_movement_system)
        .add_system(update_texture_system)
        .add_system(simulate_system)
        .add_system(modify_grid_system)
        .run();
}

fn add_grid_startup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(FlyCamera2d::default());

    let mut data = Array2::from_elem(
        [GRID_SIZE_WIDTH as usize, GRID_SIZE_HEIGHT as usize],
        Cell::Air,
    );
    let mut rng = rand::thread_rng();
    for y in 0..GRID_SIZE_HEIGHT {
        for x in 0..GRID_SIZE_WIDTH {
            let water = rng.gen::<f32>() > 0.9;
            data[[x as usize, y as usize]] = if water {
                Cell::Water(WaterData::default())
            } else {
                Cell::Air
            };
        }
    }

    let img = Image::new_fill(
        Extent3d {
            width: GRID_SIZE_WIDTH,
            height: GRID_SIZE_HEIGHT,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
    );

    let generated = images.add(img);
    commands
        .spawn(SpriteBundle {
            texture: generated,
            ..default()
        })
        .insert(Simulation { data: data.clone() });
}

fn update_texture_system(
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&Simulation, &Handle<Image>)>,
) {
    for (sim, handle) in query.iter_mut() {
        let sim: &Simulation = sim;
        let image = images.get_mut(handle).unwrap();

        let mut image_data = Vec::new();
        for y in 0..GRID_SIZE_HEIGHT {
            for x in 0..GRID_SIZE_WIDTH {
                let col = sim.data[[x as usize, y as usize]].color();
                let col = col.as_rgba_u32();
                let bytes: [u8; 4] = col.to_le_bytes();
                image_data.push(bytes[2]);
                image_data.push(bytes[1]);
                image_data.push(bytes[0]);
                image_data.push(bytes[3]);
            }
        }

        image.data = image_data;
    }
}

fn simulate_system(mut query: Query<(&mut Simulation,)>) {
    let mut rng = rand::thread_rng();
    for (sim,) in &mut query {
        let sim: &mut Simulation = sim.into_inner();
        let mut coords = (0..GRID_SIZE_HEIGHT as i32)
            .flat_map(|x| (0..GRID_SIZE_HEIGHT as i32).map(move |y| (x, y)))
            .collect::<Vec<(i32, i32)>>();
        coords.shuffle(&mut rng);

        for &(x, y) in coords.iter() {
            rule(
                SimSquareRef {
                    sim: &mut sim.data,
                    start_coord_x: x - SIM_SQUARE_SIZE as i32 / 2,
                    start_coord_y: y - SIM_SQUARE_SIZE as i32 / 2,
                },
                &mut rng,
            );
        }

        for &(x, y) in coords.iter() {
            sim.data[[x as usize, y as usize]]
                .water_mut()
                .map(|x| x.dirty = false);
        }
    }
}

fn rule(mut state: SimSquareRef<'_>, rng: &mut impl Rng) {
    let x: i32 = SIM_SQUARE_SIZE as i32 / 2;
    let y: i32 = SIM_SQUARE_SIZE as i32 / 2;
    let mut curr = if let Cell::Water(water) = state.get(x, y) {
        water
    } else {
        return;
    };
    if curr.dirty {
        return;
    }

    (|| {
        let horiz_vel_rnd: i32 = rng.gen_range(0..=1) * 2 - 1;
        // fall down
        if let Cell::Air = state.get(x, y + 1) {
            curr.vel_y += 1;
            return;
        }

        // fall diagonally
        if let Cell::Air = state.get(x + horiz_vel_rnd, y + 1) {
            curr.vel_y += 1;
            curr.vel_x = horiz_vel_rnd as i8;
            return;
        }

        // slide horizontally
        let vel_x = if curr.vel_x == 0 {
            horiz_vel_rnd * 4
        } else {
            curr.vel_x as i32
        };
        if let Cell::Air = state.get(x + horiz_vel_rnd, y) {
            curr.vel_x = vel_x as i8;
        } else {
            curr.vel_x = -curr.vel_x;
        }
    })();

    let vec = Vec2::new(curr.vel_x as f32, curr.vel_y as f32)
        .clamp_length_max((SIM_SQUARE_SIZE / 2) as f32);
    curr.vel_x = vec.x.round() as i8;
    curr.vel_y = vec.y.round() as i8;

    let first_air_cell = empty_on_line(x, y, curr.vel_x as i32, curr.vel_y as i32, &state);

    curr.vel_x = (first_air_cell.0 - x) as i8;
    curr.vel_y = (first_air_cell.1 - y) as i8;
    curr.dirty = true;

    // move with velocity
    state.get_mut(x, y).map(|x| *x = Cell::Air);
    state
        .get_mut(x + curr.vel_x as i32, y + curr.vel_y as i32)
        .map(|x| *x = Cell::Water(curr));
}

fn empty_on_line(x: i32, y: i32, vel_x: i32, vel_y: i32, state: &SimSquareRef) -> (i32, i32) {
    let mut prev_cell = (x, y);
    for p @ (p_x, p_y) in Supercover::new((x, y), (x + vel_x, y + vel_y)).skip(1) {
        if let Cell::Air = state.get(p_x, p_y) {
            prev_cell = p;
        } else {
            break;
        }
    }
    prev_cell
}

const SIM_SQUARE_SIZE: usize = 7;
pub struct SimSquareRef<'a> {
    pub sim: &'a mut Array2<Cell>,
    pub start_coord_x: i32,
    pub start_coord_y: i32,
}

impl<'a> SimSquareRef<'a> {
    pub fn get(&self, x: i32, y: i32) -> Cell {
        assert!(x < SIM_SQUARE_SIZE as i32);
        assert!(y < SIM_SQUARE_SIZE as i32);
        let x = self.start_coord_x + x;
        let y = self.start_coord_y + y;
        if x < 0 || x >= GRID_SIZE_WIDTH as i32 || y < 0 || y >= GRID_SIZE_HEIGHT as i32 {
            Cell::Solid
        } else {
            self.sim[[(x) as usize, (y) as usize]]
        }
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Cell> {
        assert!(x < SIM_SQUARE_SIZE as i32);
        assert!(y < SIM_SQUARE_SIZE as i32);
        let x = self.start_coord_x + x;
        let y = self.start_coord_y + y;
        if x < 0 || x >= GRID_SIZE_WIDTH as i32 || y < 0 || y >= GRID_SIZE_HEIGHT as i32 {
            None
        } else {
            Some(&mut self.sim[[(x) as usize, (y) as usize]])
        }
    }
}
