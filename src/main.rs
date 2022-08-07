mod components;
mod fly_camera;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use components::{Cell, Simulation};
use fly_camera::{camera_2d_movement_system, FlyCamera2d};
use ndarray::Array2;
use rand::Rng;

const GRID_SIZE: u32 = 512;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(add_grid)
        .add_system(camera_2d_movement_system)
        .add_system(update_texture)
        .add_system(simulate)
        .run();
}

fn add_grid(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert(FlyCamera2d::default());

    let mut data = Array2::from_elem([GRID_SIZE as usize, GRID_SIZE as usize], Cell::Solid);
    let mut rng = rand::thread_rng();
    for y in 0..GRID_SIZE {
        for x in 0..GRID_SIZE {
            let fill = rng.gen::<u8>();
            data[[x as usize, y as usize]] = Cell::Water { fill };
        }
    }

    let img = Image::new_fill(
        Extent3d {
            width: GRID_SIZE,
            height: GRID_SIZE,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
    );

    let generated = images.add(img);
    commands
        .spawn_bundle(SpriteBundle {
            texture: generated,
            ..default()
        })
        .insert(Simulation {
            data: data.clone(),
            double_buffer: data,
        });
}

fn update_texture(
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&Simulation, &Handle<Image>)>,
) {
    for (sim, handle) in query.iter_mut() {
        let sim: &Simulation = sim;
        let image = images.get_mut(&handle).unwrap();

        let mut image_data = Vec::new();
        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
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

fn simulate(mut query: Query<(&mut Simulation,)>) {
    for (sim,) in &mut query {
        let sim: &mut Simulation = sim.into_inner();
        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let mut state = [[Cell::Solid; 3]; 3];

                for yy in -1..=1 {
                    for xx in -1..=1 {
                        let pos_x = x as i32 + xx;
                        let pos_y = y as i32 + yy;
                        let value = if pos_x < 0
                            || pos_x >= GRID_SIZE as i32
                            || pos_y < 0
                            || pos_y >= GRID_SIZE as i32
                        {
                            Cell::Solid
                        } else {
                            *sim.data.get([pos_x as usize, pos_y as usize]).unwrap()
                        };
                        state[(xx + 1) as usize][(yy + 1) as usize] = value;
                    }
                }

                sim.double_buffer[[x as usize, y as usize]] = rule(state);
            }
        }

        std::mem::swap(&mut sim.data, &mut sim.double_buffer);
    }
}

fn rule(state: [[Cell; 3]; 3]) -> Cell {
    const MAX_FILL: u8 = 255;

    let mut curr_cell = state[1][1];

    match &mut curr_cell {
        Cell::Solid => {}
        Cell::Water { fill } => {
            let mut new_fill = *fill;
            if let Cell::Water { fill: above_fill } = state[1][0] {
                new_fill += (MAX_FILL - *fill).min(above_fill);
            }

            let mut flows_downward = false;
            if let Cell::Water { fill: below_fill } = state[1][2] {
                let flows_down = (MAX_FILL - below_fill).min(*fill);
                new_fill -= flows_down;

                flows_downward = flows_down > 0;
            }

            if !flows_downward {
                match (state[0][1], state[2][1]) {
                    (Cell::Water { fill: left_fill }, Cell::Water { fill: right_fill }) => {
                        let sum = *fill as u32 + left_fill as u32 + right_fill as u32;
                        let avg = sum / 3;
                        let rem = sum % 3;

                        new_fill = avg as u8;
                    }
                    (Cell::Water { fill: left_fill }, Cell::Solid) => {
                        let sum = *fill as u32 + left_fill as u32;
                        let avg = sum / 2;
                        let rem = sum % 2;

                        new_fill = (avg + rem) as u8;
                    }
                    (Cell::Solid, Cell::Water { fill: right_fill }) => {
                        let sum = *fill as u32 + right_fill as u32;
                        let avg = sum / 2;
                        let rem = sum % 2;

                        new_fill = (avg + rem) as u8;
                    }
                    (Cell::Solid, Cell::Solid) => {}
                }
            }

            *fill = new_fill;
        }
    }

    curr_cell
}
