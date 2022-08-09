use bevy::prelude::*;
use ndarray::Array2;

pub const GRID_SIZE_WIDTH: u32 = 32;
pub const GRID_SIZE_HEIGHT: u32 = 64;
pub const MAX_FILL: i16 = 32;

#[derive(Debug, Component)]
pub struct Simulation {
    pub data: Array2<Cell>,
    pub double_buffer: Array2<Cell>,
}

#[derive(Debug, Clone, Copy)]
pub enum Cell {
    Solid,
    Water(WaterData),
}

impl Cell {
    pub fn color(&self) -> Color {
        match self {
            Cell::Solid => todo!(),
            Cell::Water(WaterData { fill: water, .. }) => {
                if water > &0 {
                    let col_dark = Vec3::new(3. / 255., 2. / 255., 6. / 255.);
                    let col_light = Vec3::new(54. / 255., 181. / 255., 245. / 255.);
                    let interp = *water as f32 / (MAX_FILL as f32 * 2.);
                    let col = col_light.lerp(col_dark, interp);

                    Color::rgb(col.x, col.y, col.z)
                } else {
                    Color::rgba(0., 0., 0., 0.)
                }
            }
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

#[derive(Debug, Clone, Copy)]
pub struct WaterData {
    pub fill: i16,
    pub inertia_horiz: i16,
    pub inertia_vert: i16,
}
