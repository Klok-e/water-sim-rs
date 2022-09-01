use bevy::prelude::*;
use ndarray::Array2;

pub const GRID_SIZE_WIDTH: u32 = 200;
pub const GRID_SIZE_HEIGHT: u32 = 200;
pub const MAX_FILL: i16 = 32;

#[derive(Debug, Component)]
pub struct Simulation {
    pub data: Array2<Cell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Solid,
    Water(WaterData),
    Air,
}

impl Cell {
    pub fn color(&self) -> Color {
        match self {
            Cell::Solid => todo!(),
            Cell::Water(WaterData { .. }) => {
                let col_dark = Vec3::new(3. / 255., 2. / 255., 6. / 255.);
                let col_light = Vec3::new(54. / 255., 181. / 255., 245. / 255.);
                let interp = 10_f32 / (MAX_FILL as f32 * 3.);
                let col = col_light.lerp(col_dark, interp);

                Color::rgb(col.x, col.y, col.z)
            }
            Cell::Air => Color::rgba(0., 0., 0., 0.),
        }
    }

    pub fn water(&self) -> Option<&WaterData> {
        match self {
            Cell::Water(water) => Some(water),
            _ => None,
        }
    }

    pub fn water_mut(&mut self) -> Option<&mut WaterData> {
        match self {
            Cell::Water(water) => Some(water),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaterData {
    pub vel_x: i8,
    pub vel_y: i8,
    pub dirty: bool,
}
