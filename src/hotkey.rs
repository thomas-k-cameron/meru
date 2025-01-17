use bevy::prelude::*;
use enum_iterator::{all, Sequence};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::{
    app::{AppState, ShowMessage, UiState, WindowControlEvent},
    config::Config,
    core::Emulator,
    input::{InputState, KeyConfig},
};

pub struct HotKeyPlugin;

impl Plugin for HotKeyPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_hotkey)
            .add_system(process_hotkey)
            .add_event::<HotKey>()
            .insert_resource(IsTurbo(false));
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, Sequence)]
pub enum HotKey {
    Reset,
    Turbo,
    StateSave,
    StateLoad,
    NextSlot,
    PrevSlot,
    Rewind,
    Menu,
    FullScreen,
    ScaleUp,
    ScaleDown,
}

impl Display for HotKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HotKey::Reset => "Reset",
            HotKey::Turbo => "Turbo",
            HotKey::StateSave => "State Save",
            HotKey::StateLoad => "State Load",
            HotKey::NextSlot => "State Slot Next",
            HotKey::PrevSlot => "State Slot Prev",
            HotKey::Rewind => "Start Rewindng",
            HotKey::Menu => "Enter/Leave Menu",
            HotKey::FullScreen => "Fullsceen",
            HotKey::ScaleUp => "Window Scale +",
            HotKey::ScaleDown => "Window Scale -",
        };
        write!(f, "{s}")
    }
}

pub type HotKeys = KeyConfig<HotKey>;

impl Default for HotKeys {
    fn default() -> Self {
        use meru_interface::key_assign::*;
        use HotKey::*;
        Self(vec![
            (Reset, all![keycode!(LControl), keycode!(R)]),
            (Turbo, any![keycode!(Tab), pad_button!(0, LeftTrigger2)]),
            (StateSave, all![keycode!(LControl), keycode!(S)]),
            (StateLoad, all![keycode!(LControl), keycode!(L)]),
            (NextSlot, all![keycode!(LControl), keycode!(N)]),
            (PrevSlot, all![keycode!(LControl), keycode!(P)]),
            (
                Rewind,
                any![
                    keycode!(Back),
                    all![pad_button!(0, LeftTrigger2), pad_button!(0, RightTrigger2)]
                ],
            ),
            (Menu, keycode!(Escape)),
            (FullScreen, all![keycode!(RAlt), keycode!(Return)]),
            (
                ScaleUp,
                all![keycode!(LControl), any![keycode!(Plus), keycode!(Equals)]],
            ),
            (ScaleDown, all![keycode!(LControl), keycode!(Minus)]),
        ])
    }
}

pub struct IsTurbo(pub bool);

fn check_hotkey(
    config: Res<Config>,
    input_keycode: Res<Input<KeyCode>>,
    input_gamepad_button: Res<Input<GamepadButton>>,
    input_gamepad_axis: Res<Axis<GamepadAxis>>,
    mut writer: EventWriter<HotKey>,
    mut is_turbo: ResMut<IsTurbo>,
) {
    let input_state = InputState::new(&input_keycode, &input_gamepad_button, &input_gamepad_axis);

    for hotkey in all::<HotKey>() {
        if config.hotkeys.just_pressed(&hotkey, &input_state) {
            writer.send(hotkey);
        }
    }

    is_turbo.0 = config.hotkeys.pressed(
        &HotKey::Turbo,
        &InputState::new(&input_keycode, &input_gamepad_button, &input_gamepad_axis),
    );
}

fn process_hotkey(
    mut config: ResMut<Config>,
    mut reader: EventReader<HotKey>,
    mut app_state: ResMut<State<AppState>>,
    mut emulator: Option<ResMut<Emulator>>,
    mut ui_state: ResMut<UiState>,
    mut window_control_event: EventWriter<WindowControlEvent>,
    mut message_event: EventWriter<ShowMessage>,
) {
    for hotkey in reader.iter() {
        match hotkey {
            HotKey::Reset => {
                if let Some(emulator) = &mut emulator {
                    emulator.reset();
                    message_event.send(ShowMessage("Reset machine".to_string()));
                }
            }
            HotKey::StateSave => {
                if let Some(emulator) = &emulator {
                    emulator
                        .save_state_slot(ui_state.state_save_slot, config.as_ref())
                        .unwrap();
                    message_event.send(ShowMessage(format!(
                        "State saved: #{}",
                        ui_state.state_save_slot
                    )));
                }
            }
            HotKey::StateLoad => {
                if let Some(emulator) = &mut emulator {
                    if let Err(e) =
                        emulator.load_state_slot(ui_state.state_save_slot, config.as_ref())
                    {
                        message_event.send(ShowMessage("Failed to load state".to_string()));
                        error!("Failed to load state: {}", e);
                    } else {
                        message_event.send(ShowMessage(format!(
                            "State loaded: #{}",
                            ui_state.state_save_slot
                        )));
                    }
                }
            }
            HotKey::NextSlot => {
                ui_state.state_save_slot += 1;
                message_event.send(ShowMessage(format!(
                    "State slot changed: #{}",
                    ui_state.state_save_slot
                )));
            }
            HotKey::PrevSlot => {
                ui_state.state_save_slot = ui_state.state_save_slot.saturating_sub(1);
                message_event.send(ShowMessage(format!(
                    "State slot changed: #{}",
                    ui_state.state_save_slot
                )));
            }
            HotKey::Rewind => {
                if app_state.current() == &AppState::Running {
                    let emulator = emulator.as_mut().unwrap();
                    emulator.push_auto_save();
                    app_state.push(AppState::Rewinding).unwrap();
                }
            }
            HotKey::Menu => {
                if app_state.current() == &AppState::Running {
                    app_state.set(AppState::Menu).unwrap();
                } else if app_state.current() == &AppState::Menu && emulator.is_some() {
                    app_state.set(AppState::Running).unwrap();
                }
            }
            HotKey::FullScreen => {
                window_control_event.send(WindowControlEvent::ToggleFullscreen);
            }
            HotKey::ScaleUp => {
                config.scaling += 1;
                window_control_event.send(WindowControlEvent::Restore);
            }
            HotKey::ScaleDown => {
                config.scaling = (config.scaling - 1).max(1);
                window_control_event.send(WindowControlEvent::Restore);
            }

            HotKey::Turbo => {}
        }
    }
}
