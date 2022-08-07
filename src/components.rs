use bevy::prelude::*;
use ndarray::Array2;

pub const GRID_SIZE: u32 = 512;

#[derive(Debug, Component)]
pub struct Simulation {
    pub data: SimDataWrap,
}

#[derive(Debug, Clone)]
pub struct SimDataWrap(pub Array2<Cell>);

impl SimDataWrap {
    pub fn get(&self, index: [i32; 2]) -> &Cell {
        if index[0] < 0
            || index[0] >= GRID_SIZE as i32
            || index[1] < 0
            || index[1] >= GRID_SIZE as i32
        {
            &Cell::Solid
        } else {
            self.0
                .get([index[0] as usize, index[1] as usize])
                .unwrap_or(&Cell::Solid)
        }
    }
    pub fn get_mut(&mut self, index: [i32; 2]) -> &mut Cell {
        if index[0] < 0
            || index[0] >= GRID_SIZE as i32
            || index[1] < 0
            || index[1] >= GRID_SIZE as i32
        {
            &mut Cell::Solid
        } else {
            self.0
                .get_mut([index[0] as usize, index[1] as usize])
                .unwrap_or(&mut Cell::Solid)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Cell {
    Solid,
    Water { fill: u8 },
}

impl Cell {
    pub fn color(&self) -> Color {
        match self {
            Cell::Solid => todo!(),
            Cell::Water { fill: water } => {
                if water > &0 {
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
