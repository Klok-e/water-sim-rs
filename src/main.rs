mod components;
mod fly_camera;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use components::{Cell, Simulation, WaterData, GRID_SIZE, MAX_FILL};
use fly_camera::{camera_2d_movement_system, FlyCamera2d};
use ndarray::Array2;
use rand::{Rng, RngCore};

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
            let fill = (rng.gen::<f32>() * MAX_FILL as f32 / 2.) as i16;
            data[[x as usize, y as usize]] = Cell::Water(WaterData { fill });
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
    let mut rng = rand::thread_rng();
    for (sim,) in &mut query {
        let sim: &mut Simulation = sim.into_inner();
        sim.double_buffer = sim.data.clone();
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

                let changes = rule(state, &mut rng);

                for yy in -1..=1 {
                    for xx in -1..=1 {
                        let pos_x = x as i32 + xx;
                        let pos_y = y as i32 + yy;
                        if pos_x < 0
                            || pos_x >= GRID_SIZE as i32
                            || pos_y < 0
                            || pos_y >= GRID_SIZE as i32
                        {
                            continue;
                        }

                        if let Some(water) =
                            sim.double_buffer[[pos_x as usize, pos_y as usize]].water_mut()
                        {
                            water.fill =
                                (water.fill + changes[(xx + 1) as usize][(yy + 1) as usize]);

                            assert!(water.fill >= 0);
                        }
                    }
                }
            }
        }

        std::mem::swap(&mut sim.data, &mut sim.double_buffer);
    }
}

fn rule(state: [[Cell; 3]; 3], rng: &mut impl RngCore) -> [[i16; 3]; 3] {
    fn flow_to_adjacent(
        adjacent_fill: i16,
        curr_water: &mut WaterData,
        changes: &mut [[i16; 3]; 3],
        x: usize,
        y: usize,
        adj: i32,
    ) {
        if adjacent_fill < curr_water.fill && curr_water.fill > 0 {
            let can_flow = curr_water.fill - adjacent_fill / 2;

            curr_water.fill -= can_flow;
            changes[x][y] -= can_flow;
            changes[(x as i32 + adj) as usize][y] += can_flow;
        }
    }

    let x = 1usize;
    let y = 1usize;
    let mut changes = [[0i16; 3]; 3];
    let curr_cell = state[1][1];

    let mut curr_water = if let Cell::Water(curr_water) = curr_cell {
        curr_water
    } else {
        return changes;
    };

    if let Cell::Water(WaterData { fill: below_fill }) = state[x][y + 1] {
        if below_fill < MAX_FILL && curr_water.fill > 0 {
            let flow_down = (MAX_FILL - below_fill).min(curr_water.fill);
            curr_water.fill -= flow_down;
            changes[x][y] -= flow_down;
            changes[x][y + 1] += flow_down;
        }
    }

    if let Cell::Water(WaterData { fill: _ }) = state[1][0] {
        if curr_water.fill > MAX_FILL && curr_water.fill > 0 {
            let flow_up = curr_water.fill - MAX_FILL;
            curr_water.fill -= flow_up;
            changes[x][y] -= flow_up;
            changes[x][y - 1] += flow_up;
        }
    }
    match (state[0][1], state[2][1]) {
        (
            Cell::Water(WaterData { fill: left_fill }),
            Cell::Water(WaterData { fill: right_fill }),
        ) => {
            let left = rng.gen::<bool>();

            if left {
                flow_to_adjacent(left_fill, &mut curr_water, &mut changes, x, y, -1);
                flow_to_adjacent(right_fill, &mut curr_water, &mut changes, x, y, 1);
            } else {
                flow_to_adjacent(right_fill, &mut curr_water, &mut changes, x, y, 1);
                flow_to_adjacent(left_fill, &mut curr_water, &mut changes, x, y, -1);
            }
        }
        (Cell::Water(WaterData { fill: left_fill }), Cell::Solid) => {
            flow_to_adjacent(left_fill, &mut curr_water, &mut changes, x, y, -1);
        }
        (Cell::Solid, Cell::Water(WaterData { fill: right_fill })) => {
            flow_to_adjacent(right_fill, &mut curr_water, &mut changes, x, y, 1);
        }
        (Cell::Solid, Cell::Solid) => {}
    }

    changes
}
