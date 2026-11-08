# XQuest 2.1

A port of Mark Mackey's DOS game XQuest 2 (1994-1996) from Turbo Pascal to Rust and
[Bevy](https://bevy.org), for the desktop and the web. The original source code and data
files are in `xquest2_source`.

## Running

Desktop:

    cargo run --release

Web (needs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` matching the
`wasm-bindgen` version in `Cargo.lock`):

    ./build-web.sh
    python3 -m http.server -d web

On the web, click the game to capture the mouse. Press F11 to play full screen (Alt+Enter
also works on the web). The game saves its settings (`xquest.ron`), high scores
(`scores.ron`) and recorded demos (`*.demo`) as RON text: on the desktop in the current
directory and on the web in the browser's local storage.

## Controls

As in the original: move the ship with the mouse, left button fires and right button (or
space) fires a smart bomb. The keyboard can be selected in Options. `P` pauses, `Esc` quits
the current game, `S` toggles sound and `+`/`-` change the volume. If you leave the menu
alone for 30 seconds the demo plays.

## How it works

The aim is to behave identically to the original, so the game rules, timings and even the
order in which random numbers are drawn follow the original code.

- `src/core/vga.rs` models the VGA "Mode X" video memory, with its two 392x320 pages and the
  split screen used for the status bar. The game draws, saves and restores sprite
  backgrounds as the original did, so its graphics (including its quirks) come out the same.
- `src/core/game` holds the game state and rules and `src/core/ui` the menus and other
  screens. Enemy kinds (`enemies.rs`) and levels (`levels.rs`) are typed tables. The
  original's 16-bit integer overflow is reproduced where it can happen (`rng::wrap16`), and
  `src/core/rng.rs` uses the same random number generator, so recorded demos stay in sync.
- The original is written as loops that wait for the vertical retrace. The port keeps this
  structure by running the game as an `async` function that yields once per video frame
  (`src/core/platform.rs`); the Bevy front end (`src/main.rs`) polls it 60 times a second,
  shows the screen as a texture, passes on keyboard and mouse input, and plays the output of
  the mixer (`src/core/sound.rs`).
- `assets` contains the graphics as palette-indexed PNGs (pixel values are VGA colour
  indices), the sounds as WAV files and the built-in demo as RON. They are embedded in the
  binary by `src/core/assets.rs`. `src/bin/convert_assets.rs` created them from the
  original data files in `xquest2_source` (`cargo run --bin convert_assets`); it also
  checks the enemy table against the original.

## Verification against the original

The original `xquest.exe` was run in DOSBox and screenshots of its attract-mode demo were
compared with the port playing the same demo (`src/bin/headless.rs`, which runs the game
without a window). 361 of the screenshots, spread over all 12 levels of the demo and the
game over screen, are pixel-identical to a frame of the port. The other in-game
screenshots differ only where the original writes directly to the page being displayed
(the enemy entry gates, erasing explosions, the status bar), which shows up part way
through a frame depending on the speed of the computer. Menu screenshots differ because the
starfield is seeded from the clock. `tests/demo.rs` checks the game state at several of the
verified frames.

    cargo run --release --bin headless -- demo out_dir 600 1200   # write frames as PPM
    cargo test --release

## Differences from the original

- There is no joystick support (the original's joystick code reports "Joystick not found").
- The sound menu has no Sound Blaster port, IRQ and DMA settings.
- Settings, high scores and demos are stored as RON rather than the original file formats.
