mod components;
mod fly_camera;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use components::{Cell, SimDataWrap, Simulation, GRID_SIZE};
use fly_camera::{camera_2d_movement_system, FlyCamera2d};
use ndarray::Array2;
use rand::Rng;

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
            data: SimDataWrap(data),
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
                let col = sim.data.get([x as usize, y as usize]).color();
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
        let mut double_buffer = sim.data.clone();

        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                rule(&mut sim.data, &mut double_buffer, [x as i32, y as i32]);
            }
        }

        std::mem::swap(&mut sim.data, &mut double_buffer);
    }
}

fn rule(state: &mut SimDataWrap, double_buffer: &mut SimDataWrap, pos: [i32; 2]) {
    const MAX_FILL: u8 = 128;

    let mut curr_cell = state.get(pos);

    match &mut curr_cell {
        Cell::Solid => {}
        Cell::Water { fill } => {
            let mut new_fill = *fill;
            if let Cell::Water { fill: below_fill } = state.get_mut([pos[0], pos[1] - 1]) {
                let flows_down = (MAX_FILL - *below_fill).min(new_fill);
                new_fill -= flows_down;
                *below_fill += flows_down;
            }

            match (state[0][1], state[2][1]) {
                (Cell::Water { fill: left_fill }, Cell::Water { fill: right_fill }) => {
                    if left_fill < *fill && *fill > right_fill {}
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

            *fill = new_fill;
        }
    }
}
