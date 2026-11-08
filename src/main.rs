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
use bevy::window::{
    CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, WindowMode, WindowResized,
};

use xquest::core::game::XQuest;
use xquest::core::platform::{FRAMES_PER_SECOND, FrameExecutor, InputEvent, Shared, SharedRef};
use xquest::core::sound::{MIX_SPEED, MixerRef};
use xquest::core::storage;
use xquest::core::vga::{SCREEN_HEIGHT, SCREEN_WIDTH};

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
        MixerDecoder {
            mixer: self.mixer.clone(),
            buffer: vec![0.0; 64],
            pos: 64,
        }
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
        .add_systems(
            Update,
            (
                show_frame,
                fit_screen,
                grab_mouse,
                toggle_fullscreen,
                exit_when_finished,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut audio: ResMut<Assets<MixerAudio>>,
) {
    commands.spawn(Camera2d);

    let image = Image::new_fill(
        Extent3d {
            width: SCREEN_WIDTH as u32,
            height: SCREEN_HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);

    commands.spawn((Sprite::from_image(handle.clone()), ScreenSprite));
    commands.insert_resource(Screen(handle));

    let shared = Arc::new(Mutex::new(Shared::default()));
    let mixer = XQuest::new_mixer();
    let xq = XQuest::new(shared.clone(), mixer.clone(), storage::random_seed());

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
    shared.mouse_motion.0 += whole.x as i32;
    shared.mouse_motion.1 += whole.y as i32;

    for k in keys.read() {
        let pressed = k.state == ButtonState::Pressed;
        let text = k.text.as_ref().and_then(|t| t.chars().next());
        shared.events.push(InputEvent::Key {
            key: k.key_code,
            pressed,
            text,
        });
    }

    for b in buttons.read() {
        let pressed = b.state == ButtonState::Pressed;
        shared.events.push(InputEvent::MouseButton {
            button: b.button,
            pressed,
        });
    }
}

fn show_frame(mut game: ResMut<Game>, screen: Res<Screen>, mut images: ResMut<Assets<Image>>) {
    let shared = game.shared.clone();
    let shared = shared.lock().unwrap();
    if shared.frame_serial == game.last_serial {
        return;
    }
    game.last_serial = shared.frame_serial;

    if let Some(mut image) = images.get_mut(&screen.0)
        && let Some(data) = image.data.as_mut()
    {
        data.copy_from_slice(&shared.frame);
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
    let scale = (w / SCREEN_WIDTH as f32).min(h / SCREEN_HEIGHT as f32);
    sprite.custom_size = Some(Vec2::new(
        SCREEN_WIDTH as f32 * scale,
        SCREEN_HEIGHT as f32 * scale,
    ));
}

/// Grabs the mouse while the game wants relative motion. Browsers only allow this in
/// response to a click, so there it waits for one.
fn grab_mouse(
    game: Res<Game>,
    mut cursor: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
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
fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
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
