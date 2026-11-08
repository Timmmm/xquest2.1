//! Runs the game without a window, for testing.
//!
//! `headless demo <out_dir> [frames...]` plays the built-in demo and writes the listed
//! frames (or every 60th frame) as PPM images, printing the game status of each frame.
//!
//! `headless match <ref_dir>` plays the demo and reports which frame (if any) exactly matches
//! each raw 320x240 RGB reference image (`*.rgb`) in `ref_dir`.

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

fn hash(rgb: impl Iterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in rgb {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("match") {
        match_references(&args[2]);
        return;
    }

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

fn match_references(ref_dir: &str) {
    const PLAYFIELD: usize = 217 * PHYSICAL_WIDTH * 3;
    let mut refs: Vec<(String, u64, u64)> = std::fs::read_dir(ref_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "rgb").unwrap_or(false))
        .map(|p| {
            let d = std::fs::read(&p).unwrap();
            let top = hash(d[..PLAYFIELD].iter().copied());
            (p.file_name().unwrap().to_string_lossy().to_string(), hash(d.into_iter()), top)
        })
        .collect();
    refs.sort();

    let shared = Arc::new(Mutex::new(Shared::default()));
    let mut xq = Xq::new(shared.clone(), Xq::new_mixer(), 0);
    let mut exec = FrameExecutor::new(async move {
        xq.initialise_variables();
        xq.set_demo_file("xquest.dmo");
        xq.play_demo().await;
    });

    let mut found = vec![None; refs.len()];
    let mut found_top = vec![None; refs.len()];
    let mut frame = 0u64;
    while exec.step() && frame < 30000 {
        let s = shared.lock().unwrap();
        let h = hash(s.frame.chunks(4).flat_map(|p| [p[0], p[1], p[2]]));
        let top = hash(s.frame[..217 * PHYSICAL_WIDTH * 4].chunks(4).flat_map(|p| [p[0], p[1], p[2]]));
        for (i, r) in refs.iter().enumerate() {
            if r.1 == h && found[i].is_none() {
                found[i] = Some((frame, s.status));
            }
            if r.2 == top && found_top[i].is_none() {
                found_top[i] = Some(frame);
            }
        }
        frame += 1;
    }

    let mut matched = 0;
    let mut matched_top = 0;
    for ((r, f), t) in refs.iter().zip(found).zip(found_top) {
        if t.is_some() {
            matched_top += 1;
        }
        match f {
            Some((frame, st)) => {
                matched += 1;
                println!("{} frame {frame} level {} time {} score {}", r.0, st.total_level, st.time_on_level, st.score);
            }
            None => match t {
                Some(frame) => println!("{} playfield matches frame {frame}", r.0),
                None => println!("{} no match", r.0),
            },
        }
    }
    println!("{matched} of {} references matched, {matched_top} playfields matched; demo ran {frame} frames", refs.len());
}
