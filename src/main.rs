mod components;
mod fly_camera;
mod modify_grid;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use components::{Cell, GRID_SIZE_HEIGHT, GRID_SIZE_WIDTH, MAX_FILL, Simulation, WaterData};
use fly_camera::{camera_2d_movement_system, FlyCamera2d};
use modify_grid::modify_grid_system;
use ndarray::Array2;
use rand::{Rng, RngCore};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(add_grid_startup)
        .add_system(camera_2d_movement_system)
        .add_system(update_texture_system)
        .add_system(simulate_system)
        .add_system(modify_grid_system)
        .run();
}

fn add_grid_startup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert(FlyCamera2d::default());

    let mut data = Array2::from_elem([GRID_SIZE_WIDTH as usize, GRID_SIZE_HEIGHT as usize], Cell::Solid);
    let mut rng = rand::thread_rng();
    for y in 0..GRID_SIZE_HEIGHT {
        for x in 0..GRID_SIZE_WIDTH {
            let fill = (rng.gen::<f32>() * MAX_FILL as f32 / 3.) as i16;
            data[[x as usize, y as usize]] = Cell::Water(WaterData {
                fill,
                inertia_horiz: 0,
                inertia_vert: 0,
            });
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
        .spawn_bundle(SpriteBundle {
            texture: generated,
            ..default()
        })
        .insert(Simulation {
            data: data.clone(),
            double_buffer: data,
        });
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
        sim.double_buffer = sim.data.clone();
        for y in 0..GRID_SIZE_HEIGHT {
            for x in 0..GRID_SIZE_WIDTH {
                let mut state = [[Cell::Solid; 3]; 3];

                for yy in -1..=1 {
                    for xx in -1..=1 {
                        let pos_x = x as i32 + xx;
                        let pos_y = y as i32 + yy;
                        let value = if pos_x < 0
                            || pos_x >= GRID_SIZE_WIDTH as i32
                            || pos_y < 0
                            || pos_y >= GRID_SIZE_HEIGHT as i32
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
                            || pos_x >= GRID_SIZE_WIDTH as i32
                            || pos_y < 0
                            || pos_y >= GRID_SIZE_HEIGHT as i32
                        {
                            continue;
                        }

                        if let Some(water) =
                            sim.double_buffer[[pos_x as usize, pos_y as usize]].water_mut()
                        {
                            let change = changes[(xx + 1) as usize][(yy + 1) as usize];
                            water.fill += change;
                            if change != 0 {
                                if xx < 0 {
                                    water.inertia_horiz -= change;
                                }
                                if xx > 0 {
                                    water.inertia_horiz += change;
                                }
                                if yy < 0 {
                                    water.inertia_vert -= change;
                                }
                                if yy > 0 {
                                    water.inertia_vert += change;
                                }
                                if xx == 0 && yy == 0 {
                                    // dbg!(change, water.inertia_horiz,water.inertia_vert);
                                    water.inertia_horiz = 0;
                                    water.inertia_vert = 0;
                                }
                            }
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
            let can_flow = ((curr_water.fill - adjacent_fill) / 2).max(1);

            curr_water.fill -= can_flow;
            changes[x][y] -= can_flow;
            changes[(x as i32 + adj) as usize][y] += can_flow;
        }
    }
    fn flow_by_velocity(
        curr_water: &mut WaterData,
        curr_water_speed: i16,
        state: &[[Cell; 3]; 3],
        x: usize,
        y: usize,
        dx: i32,
        dy: i32,
        changes: &mut [[i16; 3]; 3],
    ) {
        match (
            state[(x as i32 + dx) as usize][(y as i32 + dy) as usize],
            state[(x as i32 - dx) as usize][(y as i32 - dy) as usize],
        ) {
            (Cell::Water(WaterData { fill: dir_fill, .. }), _) if dir_fill < MAX_FILL => {
                let inertia = curr_water_speed - 1;

                let flow_to_dir = inertia.min(curr_water.fill).max(0);

                curr_water.fill -= flow_to_dir;
                changes[x][y] -= flow_to_dir;
                changes[(x as i32 + dx) as usize][(y as i32 + dy) as usize] += flow_to_dir;
            }
            (
                _,
                Cell::Water(WaterData {
                    fill: opposite_dir_fill,
                    ..
                }),
            ) if opposite_dir_fill >= MAX_FILL => {
                // else flow in opposite direction with velocity halved
                let inertia = curr_water_speed / 2;

                let flow_to_dir = inertia.min(curr_water.fill).max(0);

                curr_water.fill -= flow_to_dir;
                changes[x][y] -= flow_to_dir;
                changes[(x as i32 - dx) as usize][(y as i32 - dy) as usize] += flow_to_dir;
            }
            _ => (),
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
    if curr_water.fill <= 0 {
        return changes;
    }

    // fall down
    if let Cell::Water(WaterData {
        fill: below_fill, ..
    }) = state[x][y + 1]
    {
        if below_fill < MAX_FILL && curr_water.fill > 0 {
            let flow_down = (MAX_FILL - below_fill).min(curr_water.fill);
            curr_water.fill -= flow_down;
            changes[x][y] -= flow_down;
            changes[x][y + 1] += flow_down;
        }
    }

    match (state[0][1], state[2][1]) {
        (
            Cell::Water(WaterData {
                fill: left_fill, ..
            }),
            Cell::Water(WaterData {
                fill: right_fill, ..
            }),
        ) => {
            let left = rng.gen::<bool>();

            // flow to both sides
            if left_fill < curr_water.fill && right_fill < curr_water.fill && curr_water.fill > 0 {
                let expected_fill = (left_fill + right_fill + curr_water.fill) / 3;
                // dbg!(left_fill, right_fill);
                // dbg!(expected_fill);
                let expected_fill_rem = (left_fill + right_fill + curr_water.fill) % 3;
                // dbg!(expected_fill_rem);

                let mut flow_left = expected_fill - left_fill;
                let mut flow_right = expected_fill - right_fill;

                if expected_fill_rem == 2 {
                    flow_left += 1;
                    flow_right += 1
                }
                if expected_fill_rem == 1 {
                    if left {
                        flow_left += 1;
                    } else {
                        flow_right += 1;
                    }
                }

                if flow_left < 0 {
                    flow_left = 0;
                }
                if flow_right < 0 {
                    flow_right = 0;
                }

                // dbg!(curr_water.fill, flow_left, flow_right);

                curr_water.fill -= flow_left + flow_right;
                changes[x][y] -= flow_left + flow_right;
                changes[(x as i32 - 1) as usize][y] += flow_left;
                changes[(x as i32 + 1) as usize][y] += flow_right;
            }
            // flow to left
            else if left_fill <= curr_water.fill
                && right_fill > curr_water.fill
                && curr_water.fill > 0
            {
                let expected_fill = (left_fill + curr_water.fill) / 2;
                let expected_fill_rem = (left_fill + curr_water.fill) % 2;

                let mut flow_left = expected_fill - left_fill;

                flow_left += expected_fill_rem;

                if flow_left < 0 {
                    flow_left = 0;
                }

                curr_water.fill -= flow_left;
                changes[x][y] -= flow_left;
                changes[(x as i32 - 1) as usize][y] += flow_left;
            }
            // flow to right
            else if left_fill >= curr_water.fill
                && right_fill < curr_water.fill
                && curr_water.fill > 0
            {
                let expected_fill = (right_fill + curr_water.fill) / 2;
                let expected_fill_rem = (right_fill + curr_water.fill) % 2;

                let mut flow_right = expected_fill - right_fill;

                flow_right += expected_fill_rem;

                if flow_right < 0 {
                    flow_right = 0;
                }

                curr_water.fill -= flow_right;
                changes[x][y] -= flow_right;
                changes[(x as i32 + 1) as usize][y] += flow_right;
            }
        }
        (
            Cell::Water(WaterData {
                fill: left_fill, ..
            }),
            Cell::Solid,
        ) => {
            flow_to_adjacent(left_fill, &mut curr_water, &mut changes, x, y, -1);
        }
        (
            Cell::Solid,
            Cell::Water(WaterData {
                fill: right_fill, ..
            }),
        ) => {
            flow_to_adjacent(right_fill, &mut curr_water, &mut changes, x, y, 1);
        }
        (Cell::Solid, Cell::Solid) => {}
    }

    //  // move by inertia first
    //  let curr_water_speed = curr_water.inertia_vert;
    //  if curr_water_speed > 0 {
    //      flow_by_velocity(
    //          &mut curr_water,
    //          curr_water_speed,
    //          &state,
    //          x,
    //          y,
    //          0,
    //          1,
    //          &mut changes,
    //      );
    //  }
    //  if curr_water_speed < 0 {
    //      flow_by_velocity(
    //          &mut curr_water,
    //          -curr_water_speed,
    //          &state,
    //          x,
    //          y,
    //          0,
    //          -1,
    //          &mut changes,
    //      );
    //  }
    //  let curr_water_speed = curr_water.inertia_horiz;
    //  if curr_water_speed > 0 {
    //      flow_by_velocity(
    //          &mut curr_water,
    //          curr_water_speed,
    //          &state,
    //          x,
    //          y,
    //          1,
    //          0,
    //          &mut changes,
    //      );
    //  }
    //  if curr_water_speed < 0 {
    //      flow_by_velocity(
    //          &mut curr_water,
    //          -curr_water_speed,
    //          &state,
    //          x,
    //          y,
    //          -1,
    //          0,
    //          &mut changes,
    //      );
    //  }

    // bubble up because of pressure
    if let Cell::Water(WaterData {
        fill: above_fill, ..
    }) = state[x][y - 1]
    {
        if curr_water.fill > MAX_FILL && above_fill - curr_water.fill < -1 {
            let flow_up = curr_water.fill - MAX_FILL;
            dbg!(curr_water.fill,above_fill,flow_up);
            curr_water.fill -= flow_up;
            changes[x][y] -= flow_up;
            changes[x][y - 1] += flow_up;
        }
    }

    // pressure below
    if let Cell::Water(WaterData {
        fill: below_fill, ..
    }) = state[x][y + 1]
    {
        // dbg!(curr_water.fill, below_fill);
        if curr_water.fill >= MAX_FILL
            && below_fill >= MAX_FILL
            && curr_water.fill - below_fill >= 0
        {
            // dbg!("dddd", curr_water.fill, below_fill);
            curr_water.fill -= 1;
            changes[x][y] -= 1;
            changes[x][y + 1] += 1;
        }
    }

    // fall down because of pressure
    if let Cell::Water(WaterData {
        fill: below_fill, ..
    }) = state[x][y + 1]
    {
        if below_fill >= MAX_FILL && curr_water.fill >= MAX_FILL && curr_water.fill > below_fill {
            let flow_down = curr_water.fill - below_fill;
            curr_water.fill -= flow_down;
            changes[x][y] -= flow_down;
            changes[x][y + 1] += flow_down;
        }
    }

    changes
}
