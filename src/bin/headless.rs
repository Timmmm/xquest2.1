//! Runs the game without a window, for testing.
//!
//! `headless demo <out_dir> [frames...]` plays the built-in demo and writes the listed
//! frames (or every 60th frame) as PPM images, printing the game status of each frame.

use std::sync::{Arc, Mutex};

use xquest::core::game::Xq;
use xquest::core::platform::{FrameExecutor, Shared};
use xquest::core::vga::{PHYSICAL_HEIGHT, PHYSICAL_WIDTH};

fn write_ppm(path: &str, rgba: &[u8]) {
    let mut out = format!("P6\n{PHYSICAL_WIDTH} {PHYSICAL_HEIGHT}\n255\n").into_bytes();
    for px in rgba.chunks(4) {
        out.extend_from_slice(&px[..3]);
    }
    std::fs::write(path, out).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
    let frames: Vec<u64> = args.iter().skip(3).filter_map(|a| a.parse().ok()).collect();
    std::fs::create_dir_all(&out_dir).unwrap();

    let shared = Arc::new(Mutex::new(Shared::default()));
    let mixer = Xq::new_mixer();
    let mut xq = Xq::new(shared.clone(), mixer, 0);

    let mut exec = FrameExecutor::new(async move {
        xq.initialise_variables();
        xq.set_demo_file("xquest.dmo");
        xq.play_demo().await;
    });

    let max = frames.iter().copied().max().unwrap_or(20000);
    let mut frame = 0u64;
    let mut last = None;

    while exec.step() && frame <= max {
        let s = shared.lock().unwrap();
        let st = s.status;
        let key = (st.total_level, st.time_on_level, st.score, st.lives);
        if last != Some(key) {
            println!("frame {frame} level {} time {} score {} lives {} seed {:08x}", st.total_level, st.time_on_level, st.score, st.lives, st.rand_seed);
            last = Some(key);
        }
        if frames.contains(&frame) || (frames.is_empty() && frame % 60 == 0) {
            write_ppm(&format!("{out_dir}/f{frame:06}.ppm"), &s.frame);
        }
        frame += 1;
    }
}
