use bevy::prelude::*;
use ndarray::Array2;

pub const GRID_SIZE: u32 = 512;

#[derive(Debug, Component)]
pub struct Simulation {
    pub data: Array2<Cell>,
    pub double_buffer: Array2<Cell>,
}

#[derive(Debug, Clone, Copy)]
pub enum Cell {
    Solid,
    Water { fill: f32 },
}

impl Cell {
    pub fn color(&self) -> Color {
        match self {
            Cell::Solid => todo!(),
            Cell::Water { fill: water } => {
                if water > &0.1 {
                    let col = Vec3::new(0.49, 1.0, 0.83);
                    let col = col.clamp(Vec3::splat(0.), Vec3::splat(1.));
                    let col = Color::rgb(col.x, col.y, col.z);
                    col
                } else {
                    Color::rgba(0., 0., 0., 0.)
                }
            }
        }
    }
}
