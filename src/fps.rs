use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
/// FPS counter, based on implementation from https://bevy-cheatbook.github.io/cookbook/print-framerate.html
use bevy::ecs::system::Commands;
use bevy::prelude::*;

// Define colors
const TEXT_COLOR_DEFAULT: bevy::prelude::Color = Color::rgb(0.0, 1.0, 0.0);
const TEXT_COLOR_BAD: bevy::prelude::Color = Color::rgb(1.0, 0.0, 0.0);

pub fn setup_fps(mut commands: Commands) {
    // add FPS container entity
    let fpscontainer = commands
        .spawn((
            FPSContainer,
            // Container for the FPS counter
            NodeBundle {
                // Make sure the text is always on top
                z_index: ZIndex::Global(i32::MAX),
                // Position the text in the top right corner
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
                // Slightly dark background
                background_color: bevy::prelude::BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.5)),
                ..default()
            },
        ))
        .id();
    // add FPS counter text entity
    let text_fps = commands
        .spawn((
            FPSCounterText,
            TextBundle {
                // Use two sections, as to only override the FPS value
                // Small green text
                text: Text::from_sections([
                    TextSection {
                        value: "FPS: ".into(),
                        style: TextStyle {
                            font_size: 16.0,
                            color: TEXT_COLOR_DEFAULT,
                            ..default()
                        },
                    },
                    TextSection {
                        value: " N/A".into(),
                        style: TextStyle {
                            font_size: 16.0,
                            color: TEXT_COLOR_DEFAULT,
                            ..default()
                        },
                    },
                ]),
                ..Default::default()
            },
        ))
        .id();
    // Add the text entity as a child of the container
    commands.entity(fpscontainer).push_children(&[text_fps]);
}

// Marker components for the FPS counter
#[derive(Component)]
pub struct FPSContainer;
#[derive(Component)]
pub struct FPSCounterText;

pub fn update_fps(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FPSCounterText>>,
) {
    for mut text in &mut query {
        // try to get a "smoothed" FPS value from Bevy
        if let Some(value) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            // Format the number as to leave space for 3 digits, just in case,
            // right-aligned and rounded. This helps readability when the
            // number changes rapidly.
            text.sections[1].value = format!("{value:>3.0}");

            // Change the color based on the FPS value
            text.sections[1].style.color = if value >= 30.0 {
                // Above 30 FPS, which is the target spec.
                TEXT_COLOR_DEFAULT
            } else {
                // Below 30 FPS, which is below target spec.
                TEXT_COLOR_BAD
            }
        } else {
            // display "N/A" if we can't get a FPS measurement
            text.sections[1].value = "N/A".into();
            text.sections[1].style.color = TEXT_COLOR_BAD;
        }
    }
}
