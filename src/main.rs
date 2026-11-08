//! The Bevy front end: a window showing the emulated VGA screen, keyboard and mouse input,
//! and audio output. The game itself lives in [`xquest::core`].

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::audio::{AddAudioSource, ChannelCount, SampleRate, Source};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseButtonInput};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, WindowMode, WindowResized};

use xquest::core::game::Xq;
use xquest::core::platform::{FRAMES_PER_SECOND, FrameExecutor, InputEvent, Shared, SharedRef};
use xquest::core::sound::{MIX_SPEED, MixerRef};
use xquest::core::storage;
use xquest::core::vga::{PHYSICAL_HEIGHT, PHYSICAL_WIDTH};

#[derive(Resource)]
struct Game {
    shared: SharedRef,
    executor: Mutex<FrameExecutor>,
    last_serial: u64,
    /// Sub-pixel mouse motion carried over to the next frame.
    motion_remainder: Vec2,
}

#[derive(Resource)]
struct Screen(Handle<Image>);

#[derive(Component)]
struct ScreenSprite;

/// An endless audio stream produced by the game's mixer.
#[derive(Asset, TypePath)]
struct MixerAudio {
    mixer: MixerRef,
}

struct MixerDecoder {
    mixer: MixerRef,
    buffer: Vec<f32>,
    pos: usize,
}

impl Iterator for MixerDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buffer.len() {
            let mut m = self.mixer.lock().unwrap();
            for s in self.buffer.iter_mut() {
                *s = (m.next_sample() as f32 - 128.0) / 128.0;
            }
            self.pos = 0;
        }

        let s = self.buffer[self.pos];
        self.pos += 1;
        Some(s)
    }
}

impl Source for MixerDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(MIX_SPEED).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for MixerAudio {
    type Decoder = MixerDecoder;

    fn decoder(&self) -> Self::Decoder {
        MixerDecoder { mixer: self.mixer.clone(), buffer: vec![0.0; 64], pos: 64 }
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "XQuest 2".to_string(),
                        resolution: (960, 720).into(),
                        canvas: Some("#bevy".to_string()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_audio_source::<MixerAudio>()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(FRAMES_PER_SECOND))
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, collect_input.after(bevy::input::InputSystems))
        .add_systems(FixedUpdate, step_game)
        .add_systems(Update, (show_frame, fit_screen, grab_mouse, toggle_fullscreen, exit_when_finished))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut audio: ResMut<Assets<MixerAudio>>) {
    commands.spawn(Camera2d);

    let image = Image::new_fill(
        Extent3d { width: PHYSICAL_WIDTH as u32, height: PHYSICAL_HEIGHT as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);

    commands.spawn((Sprite::from_image(handle.clone()), ScreenSprite));
    commands.insert_resource(Screen(handle));

    let shared = Arc::new(Mutex::new(Shared::default()));
    let mixer = Xq::new_mixer();
    let xq = Xq::new(shared.clone(), mixer.clone(), storage::random_seed());

    commands.spawn(AudioPlayer(audio.add(MixerAudio { mixer })));
    commands.insert_resource(Game {
        shared,
        executor: Mutex::new(FrameExecutor::new(xq.run())),
        last_serial: 0,
        motion_remainder: Vec2::ZERO,
    });
}

fn step_game(game: Res<Game>) {
    game.executor.lock().unwrap().step();
}

fn collect_input(
    mut game: ResMut<Game>,
    mut keys: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    motion: Res<AccumulatedMouseMotion>,
) {
    let total = game.motion_remainder + motion.delta;
    let whole = total.trunc();
    game.motion_remainder = total - whole;

    let mut shared = game.shared.lock().unwrap();
    shared.mouse_dx += whole.x as i32;
    shared.mouse_dy += whole.y as i32;

    for k in keys.read() {
        if let Some(scan) = scan_code(k.key_code) {
            let pressed = k.state == ButtonState::Pressed;
            shared.events.push(InputEvent::Key { scan, pressed });
        }
    }

    for b in buttons.read() {
        let button = match b.button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            _ => continue,
        };
        let pressed = b.state == ButtonState::Pressed;
        shared.events.push(InputEvent::MouseButton { button, pressed });
    }
}

fn show_frame(mut game: ResMut<Game>, screen: Res<Screen>, mut images: ResMut<Assets<Image>>) {
    let shared = game.shared.clone();
    let shared = shared.lock().unwrap();
    if shared.frame_serial == game.last_serial {
        return;
    }
    game.last_serial = shared.frame_serial;

    if let Some(mut image) = images.get_mut(&screen.0) {
        if let Some(data) = image.data.as_mut() {
            data.copy_from_slice(&shared.frame);
        }
    }
}

/// Scales the screen to fill the window while keeping the 4:3 aspect ratio.
fn fit_screen(
    mut resized: MessageReader<WindowResized>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut sprite: Single<&mut Sprite, With<ScreenSprite>>,
    mut first: Local<bool>,
) {
    if resized.read().count() == 0 && *first {
        return;
    }
    *first = true;

    let (w, h) = (window.width(), window.height());
    let scale = (w / PHYSICAL_WIDTH as f32).min(h / PHYSICAL_HEIGHT as f32);
    sprite.custom_size = Some(Vec2::new(PHYSICAL_WIDTH as f32 * scale, PHYSICAL_HEIGHT as f32 * scale));
}

/// Grabs the mouse while the game wants relative motion. Browsers only allow this in
/// response to a click, so there it waits for one.
fn grab_mouse(game: Res<Game>, mut cursor: Single<&mut CursorOptions>, mouse: Res<ButtonInput<MouseButton>>) {
    let wants = game.shared.lock().unwrap().wants_mouse_grab;

    if wants && cursor.grab_mode != CursorGrabMode::Locked {
        if !cfg!(target_arch = "wasm32") || mouse.just_pressed(MouseButton::Left) {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    } else if !wants && cursor.grab_mode != CursorGrabMode::None {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// F11 toggles full screen. On the web the page itself handles this (see
/// `web/index.html`), because browsers only allow full screen in direct response to input.
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    if cfg!(target_arch = "wasm32") {
        return;
    }

    if keys.just_pressed(KeyCode::F11) {
        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            _ => WindowMode::Windowed,
        };
    }
}

fn exit_when_finished(game: Res<Game>, mut exit: MessageWriter<AppExit>) {
    if game.shared.lock().unwrap().finished {
        exit.write(AppExit::Success);
    }
}

/// Maps a key to its PC scan code. Keys sent with an `$E0` prefix have 128 added.
fn scan_code(key: KeyCode) -> Option<u8> {
    use KeyCode::*;

    Some(match key {
        Escape => 0x01,
        Digit1 => 0x02,
        Digit2 => 0x03,
        Digit3 => 0x04,
        Digit4 => 0x05,
        Digit5 => 0x06,
        Digit6 => 0x07,
        Digit7 => 0x08,
        Digit8 => 0x09,
        Digit9 => 0x0A,
        Digit0 => 0x0B,
        Minus => 0x0C,
        Equal => 0x0D,
        Backspace => 0x0E,
        Tab => 0x0F,
        KeyQ => 0x10,
        KeyW => 0x11,
        KeyE => 0x12,
        KeyR => 0x13,
        KeyT => 0x14,
        KeyY => 0x15,
        KeyU => 0x16,
        KeyI => 0x17,
        KeyO => 0x18,
        KeyP => 0x19,
        BracketLeft => 0x1A,
        BracketRight => 0x1B,
        Enter => 0x1C,
        ControlLeft => 0x1D,
        KeyA => 0x1E,
        KeyS => 0x1F,
        KeyD => 0x20,
        KeyF => 0x21,
        KeyG => 0x22,
        KeyH => 0x23,
        KeyJ => 0x24,
        KeyK => 0x25,
        KeyL => 0x26,
        Semicolon => 0x27,
        Quote => 0x28,
        Backquote => 0x29,
        ShiftLeft => 0x2A,
        Backslash => 0x2B,
        KeyZ => 0x2C,
        KeyX => 0x2D,
        KeyC => 0x2E,
        KeyV => 0x2F,
        KeyB => 0x30,
        KeyN => 0x31,
        KeyM => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        ShiftRight => 0x36,
        NumpadMultiply => 0x37,
        AltLeft => 0x38,
        Space => 0x39,
        CapsLock => 0x3A,
        F1 => 0x3B,
        F2 => 0x3C,
        F3 => 0x3D,
        F4 => 0x3E,
        F5 => 0x3F,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        NumLock => 0x45,
        ScrollLock => 0x46,
        Numpad7 => 0x47,
        Numpad8 => 0x48,
        Numpad9 => 0x49,
        NumpadSubtract => 0x4A,
        Numpad4 => 0x4B,
        Numpad5 => 0x4C,
        Numpad6 => 0x4D,
        NumpadAdd => 0x4E,
        Numpad1 => 0x4F,
        Numpad2 => 0x50,
        Numpad3 => 0x51,
        Numpad0 => 0x52,
        NumpadDecimal => 0x53,
        F11 => 0x57,
        F12 => 0x58,
        NumpadEnter => 0x9C,
        ControlRight => 0x9D,
        NumpadDivide => 0xB5,
        AltRight => 0xB8,
        Home => 0xC7,
        ArrowUp => 0xC8,
        PageUp => 0xC9,
        ArrowLeft => 0xCB,
        ArrowRight => 0xCD,
        End => 0xCF,
        ArrowDown => 0xD0,
        PageDown => 0xD1,
        Insert => 0xD2,
        Delete => 0xD3,
        _ => return None,
    })
}
