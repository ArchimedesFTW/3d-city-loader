use crate::common::StatusEvent;
use crate::data::loading::DataQueryEvent;
use crate::data::query::{parse_data_query, InputQueryType};
use crate::player::PlayerMoveEvent;
use wasm_bindgen::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::window::{CursorGrabMode, PresentMode, PrimaryWindow};

use bevy_egui::egui;
use bevy_egui::EguiContexts;

use std::collections::vec_deque::VecDeque;

#[wasm_bindgen]
extern {
    fn setup_finished();
}

/// The state of the UI, such as values for input fields, excluding the main
/// earth panel.
#[derive(Debug, Resource)]
pub struct UiState {
    pub cursor_locked: bool,
    pub query: String,
    pub query_type: InputQueryType,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            cursor_locked: false,
            query: String::new(),
            query_type: InputQueryType::City,
        }
    }
}

/// A system that sets up the UI and window.
///
/// Right now, it maximizes the window and sets a title, and adds an entity for
/// the notification text.
pub fn setup_ui(mut commands: Commands, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = windows.get_single_mut().unwrap_throw();
    window.set_maximized(true);
    window.title = "Earth Simulator".to_owned();

    // Set up presentation mode, to uncap the frame rate
    window.present_mode = PresentMode::AutoNoVsync;

    // add notification text entity
    commands.spawn((
        NotificationText {
            queue: VecDeque::new(),
        },
        TextBundle {
            text: Text {
                linebreak_behavior: BreakLineOn::WordBoundary,
                justify: JustifyText::Left,
                ..default()
            },
            style: Style {
                max_width: Val::Percent(25.0),
                position_type: PositionType::Absolute,
                top: Val::Percent(6.0),
                right: Val::Percent(1.0),
                bottom: Val::Percent(1.0),
                left: Val::Percent(74.0),
                ..default()
            },
            ..default()
        },
    ));

    // Communicate with the JavaScript that the setup is finished
    setup_finished();
}

/// A system that updates the UI for the next frame.
///
/// The UI system used is `egui` which uses immediate mode, so this is also
/// where the UI layout is drawn.
pub fn update_ui(
    // UI
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    // input events
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_input: EventReader<MouseMotion>,
    // generated events
    mut player_move_events: EventWriter<PlayerMoveEvent>,
    mut data_load_events: EventWriter<DataQueryEvent>,
    mut status_events: EventWriter<StatusEvent>,
) {
    // we only have one window, so the primary window is always used
    let mut primary_window = windows.single_mut();
    let ctx = contexts.ctx_mut();

    let window = egui::Window::new("Earth Loader Panel").id("earth_loader_panel".into());

    window.show(ctx, |ui| {
        ui.label("Enter a query to load it");

        egui::ComboBox::from_id_source("query_type")
            .selected_text(match &ui_state.query_type {
                InputQueryType::City => "City",
                InputQueryType::File => "File",
                InputQueryType::Overpass => "Overpass API",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ui_state.query_type, InputQueryType::City, "City");
                ui.selectable_value(&mut ui_state.query_type, InputQueryType::File, "File");
                ui.selectable_value(
                    &mut ui_state.query_type,
                    InputQueryType::Overpass,
                    "Overpass API",
                );
            });

        // Add the multiline text element and capture the response
        let response = ui.add(egui::TextEdit::multiline(&mut ui_state.query)
            .hint_text("Press TAB to enter city..."));

        // Set focus to the text edit if the user presses tab
        if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            response.request_focus();

            // Clear previous text
            ui_state.query.clear();
        }

        // If the user presses enter while the text edit is focused, load the data
        let submit_using_enter: bool = keyboard_input.just_pressed(KeyCode::Enter) && response.has_focus();
        if ui.button("LOAD - press ENTER").clicked() || submit_using_enter {
            // Remove \n (newline) characters from the query
            ui_state.query = ui_state.query.replace("\n", "");

            match parse_data_query(ui_state.query_type, &ui_state.query) {
                Ok(query) => {
                    status_events.send(StatusEvent::Update(
                        "Succesfully parsed query, now handling it".to_owned(),
                    ));
                    data_load_events.send(DataQueryEvent { query });
                }
                Err(error) => {
                    status_events.send(StatusEvent::Error(error));
                }
            }

            ui_state.query.clear();
        }
    });

    // see also: https://bevy-cheatbook.github.io/window/mouse-grab.html
    if ui_state.cursor_locked {
        let mut translation = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            translation += Vec3::NEG_Z;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            translation += Vec3::NEG_X;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            translation += Vec3::Z;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            translation += Vec3::X;
        }
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            translation += Vec3::NEG_Y;
        }
        if keyboard_input.pressed(KeyCode::Space) {
            translation += Vec3::Y;
        }

        // `rotation` is a Vec2 but controls rotation of the camera
        let rotation = if let Some(event) = mouse_motion_input.read().next() {
            // recenter cursor on Windows, because platform differences :/
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));

            event.delta
        } else {
            Vec2::ZERO
        };

        let do_panning = keyboard_input.pressed(KeyCode::KeyP);

        if translation != Vec3::ZERO {
            translation = translation.normalize();
        }
        if translation != Vec3::ZERO || rotation != Vec2::ZERO || do_panning {
            player_move_events.send(PlayerMoveEvent {
                translation,
                rotation,
                do_panning,
            });
        }

        if keyboard_input.just_pressed(KeyCode::Escape) {
            // unlock cursor, allowing to access UI again
            primary_window.cursor.grab_mode = CursorGrabMode::None;
            primary_window.cursor.visible = true;
            ui_state.cursor_locked = false;
        }
    }

    if !ctx.is_pointer_over_area() && mouse_button_input.just_pressed(MouseButton::Left) {
        // lock cursor, allowing to translate and rotate the camera
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
        ui_state.cursor_locked = true;
    }
}

#[derive(Component)]
pub struct NotificationText {
    pub queue: VecDeque<(String, TextStyle, Timer)>,
}

/// A system that updates the notification text in the corner
pub fn update_notifications(
    mut query: Query<(&mut NotificationText, &mut Text)>,
    time: Res<Time>,
    mut status_events: EventReader<StatusEvent>,
) {
    let (mut notifications, mut text) = query.get_single_mut().unwrap_throw();

    let mut changed = false;

    // check timers
    for (_, _, timer) in &mut notifications.queue {
        timer.tick(time.delta());
    }
    while let Some((_, _, timer)) = notifications.queue.front() {
        if timer.finished() {
            notifications.queue.pop_front();
            changed = true;
        } else {
            break;
        }
    }

    // update text sections
    for status_event in status_events.read() {
        let (text, style) = match status_event {
            StatusEvent::Error(error) => {
                let style = TextStyle {
                    color: ERROR_COLOR,
                    font_size: NOTIFICATION_FONT_SIZE,
                    ..default()
                };
                (format!("{}\n\n", error), style)
            }
            StatusEvent::Update(message) => {
                let style = TextStyle {
                    color: UPDATE_COLOR,
                    font_size: NOTIFICATION_FONT_SIZE,
                    ..default()
                };
                (format!("{}\n\n", message), style)
            }
        };
        notifications.queue.push_back((
            text,
            style,
            Timer::from_seconds(NOTIFICATION_TIME, TimerMode::Once),
        ));

        changed = true;
    }

    if changed {
        text.sections.clear();
        for notification in &notifications.queue {
            text.sections
                .push(TextSection::new(&notification.0, notification.1.clone()));
        }
    }
}

const ERROR_COLOR: Color = Color::RED;
const UPDATE_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const NOTIFICATION_FONT_SIZE: f32 = 15.0;
const NOTIFICATION_TIME: f32 = 5.0;
