use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::{AssetServer, Color, Commands, Component, Plugin, Query, Res, TextBundle, With},
    text::{Text, TextAlignment, TextSection, TextStyle},
    ui::{PositionType, Style, UiRect, Val},
};

#[derive(Component)]
pub struct FpsText;

pub fn fps_ui_system(mut text: Query<&mut Text, With<FpsText>>, diagnostics: Res<Diagnostics>) {
    let mut text = text.single_mut();
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            // Update the value of the second section
            text.sections[1].value = format!("{value:.2}");
        }
    }
}

pub struct DebugUiBundle;

impl Plugin for DebugUiBundle {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default());
        app.add_system(fps_ui_system);

        app.add_startup_system(startup);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/fira_sans/FiraSans-Bold.ttf");
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font: font.clone(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font,
                font_size: 30.0,
                color: Color::GOLD,
            }),
        ])
        .with_text_alignment(TextAlignment::Left)
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        }),
        FpsText,
    ));
}
