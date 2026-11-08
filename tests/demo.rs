//! Plays the original demo recording and checks the game state at frames whose screen was
//! found to be pixel-identical to the original game running in DOSBox.

use std::sync::{Arc, Mutex};

use xquest::core::game::Xq;
use xquest::core::platform::{FrameExecutor, Shared};

/// (frame, level, time on level, score), from matching DOSBox screenshots.
const CHECKPOINTS: [(u64, i32, i64, i64); 6] = [
    (2275, 3, 339, 19700),
    (4602, 5, 282, 37400),
    (7007, 6, 171, 42900),
    (9436, 7, 1313, 61200),
    (12010, 9, 390, 82600),
    (14568, 10, 911, 97900),
];

#[test]
fn demo_matches_original() {
    let shared = Arc::new(Mutex::new(Shared::default()));
    let mut xq = Xq::new(shared.clone(), Xq::new_mixer(), 0);

    let mut exec = FrameExecutor::new(async move {
        xq.initialise_variables();
        assert!(xq.set_demo_file("xquest.dmo"));
        xq.play_demo().await;
    });

    let mut frame = 0;
    for &(f, level, time, score) in CHECKPOINTS.iter() {
        while frame <= f {
            assert!(exec.step(), "demo ended early at frame {frame}");
            frame += 1;
        }

        let st = shared.lock().unwrap().status;
        assert_eq!((st.total_level, st.time_on_level, st.score), (level, time, score), "at frame {f}");
    }
}
