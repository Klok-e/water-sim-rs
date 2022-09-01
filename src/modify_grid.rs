use bevy::{
    input::Input,
    math::Vec2,
    prelude::{Camera, GlobalTransform, MouseButton, Query, Res, Transform},
    window::Windows,
};

use crate::components::{Cell, Simulation, WaterData, GRID_SIZE_HEIGHT, GRID_SIZE_WIDTH, MAX_FILL};

pub fn modify_grid_system(
    windows: Res<Windows>,
    mouse_buttons: Res<Input<MouseButton>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut sim: Query<(&mut Simulation, &Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let (camera, camera_transform) = camera.single();

    let wnd = windows.get_primary().unwrap();
    let screen_pos = if let Some(pos) = wnd.cursor_position() {
        pos
    } else {
        return;
    };

    // get the size of the window
    let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

    // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

    // matrix for undoing the projection and camera transform
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

    // use it to convert ndc to world-space coordinates
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

    // reduce it to a 2D value
    let world_pos: Vec2 = world_pos.truncate();

    let (mut sim, sim_pos): (_, &Transform) = sim.single_mut();
    let sim: &mut Simulation = &mut *sim;
    let sim_pos = sim_pos.translation.truncate()
        - Vec2::from([GRID_SIZE_WIDTH as f32, GRID_SIZE_HEIGHT as f32]) / 2.;
    let relative = world_pos - sim_pos;
    let (index_x, index_y) = (
        relative.x.round() as i32,
        GRID_SIZE_HEIGHT as i32 - relative.y.round() as i32,
    );

    let brush_size = 10;

    for y in -brush_size..=brush_size {
        for x in -brush_size..=brush_size {
            let (pos_x, pos_y) = (index_x + x, index_y + y);
            if pos_x < 0
                || pos_x >= GRID_SIZE_WIDTH as i32
                || pos_y < 0
                || pos_y >= GRID_SIZE_HEIGHT as i32
            {
                continue;
            }

            sim.data[[pos_x as usize, pos_y as usize]] = Cell::Water(WaterData::default());
        }
    }
}
