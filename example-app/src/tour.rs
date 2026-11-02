//! Screenshot tour driver.
//!
//! Enabled with `NAVI_TOUR_DIR=<dir>`: executes a fixed, deterministic script
//! of navigation/devtools actions against the running example app and captures
//! an X11 frame after each step with `ffmpeg(1)`. The SAME script runs in the
//! pre- and post-migration apps so the resulting screenshot series can be
//! compared 1:1.

use gpui::{App, AsyncApp, Global};
use navi_devtools::{DevtoolsTab, SharedModel};
use navi_router::Navigator;
use std::path::PathBuf;
use std::time::Duration;

/// Global handle giving the async tour driver access to the devtools model.
pub struct TourGlobal {
    pub devtools: SharedModel,
}

impl Global for TourGlobal {}

#[derive(Clone, Debug)]
pub enum TourStep {
    /// Push a location through the router (as a Link click would).
    Nav(&'static str),
    /// Wait real milliseconds (loaders, repaints).
    WaitMs(u64),
    /// Capture the current frame into `<seq>_<name>.png`.
    Shot(&'static str),
    /// Switch the devtools panel tab.
    DevtoolsTab(&'static str),
    /// Replace the timeline search text.
    DevtoolsSearch(&'static str),
    /// Toggle devtools expanded (panel <-> FAB).
    DevtoolsToggle,
}

/// The tour script. Identical (in order and timing) across pre- and
/// post-migration runs.
pub const SCRIPT: &[TourStep] = &[
    TourStep::WaitMs(2500),
    TourStep::Shot("01_home"),
    TourStep::Nav("/users"),
    TourStep::WaitMs(2000),
    TourStep::Shot("02_users"),
    TourStep::DevtoolsTab("timeline"),
    TourStep::WaitMs(400),
    TourStep::Shot("03_timeline_events"),
    TourStep::DevtoolsTab("routes"),
    TourStep::WaitMs(400),
    TourStep::Shot("04_routes_tree"),
    TourStep::DevtoolsTab("cache"),
    TourStep::WaitMs(400),
    TourStep::Shot("05_cache"),
    TourStep::DevtoolsTab("state"),
    TourStep::WaitMs(400),
    TourStep::Shot("06_state"),
    TourStep::Nav("/users/1"),
    TourStep::WaitMs(1800),
    TourStep::Shot("07_user1"),
    TourStep::DevtoolsTab("timeline"),
    TourStep::WaitMs(400),
    TourStep::Shot("08_timeline_more"),
    TourStep::DevtoolsSearch("users"),
    TourStep::WaitMs(400),
    TourStep::Shot("09_search_users"),
    TourStep::DevtoolsSearch(""),
    TourStep::WaitMs(200),
    TourStep::DevtoolsTab("routes"),
    TourStep::WaitMs(300),
    TourStep::Nav("/awaited"),
    TourStep::WaitMs(3000),
    TourStep::Shot("10_awaited"),
    TourStep::Nav("/nonexistent"),
    TourStep::WaitMs(800),
    TourStep::Shot("11_404"),
    TourStep::Nav("/settings"),
    TourStep::WaitMs(600),
    TourStep::Shot("12_settings"),
    TourStep::DevtoolsToggle,
    TourStep::WaitMs(500),
    TourStep::Shot("13_fab"),
    TourStep::DevtoolsToggle,
    TourStep::WaitMs(500),
    TourStep::Shot("14_expanded_again"),
];

fn tab_from_name(name: &str) -> DevtoolsTab {
    match name {
        "routes" => DevtoolsTab::Routes,
        "cache" => DevtoolsTab::Cache,
        "state" => DevtoolsTab::State,
        _ => DevtoolsTab::Timeline,
    }
}

fn capture_frame(out_path: &PathBuf) {
    let _ = std::fs::remove_file(out_path);
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "x11grab",
            "-video_size",
            "1280x900",
            "-i",
            &std::env::var("DISPLAY").unwrap_or_else(|_| ":99".to_string()),
            "-frames:v",
            "1",
            out_path.to_string_lossy().as_ref(),
        ])
        .status();
    if let Err(e) = status {
        eprintln!("TOUR: ffmpeg capture failed: {e}");
    }
}

/// Execute the tour. Spawned once the window is up; exits the process at the
/// end so automation runs are self-terminating.
pub async fn run_tour(cx: AsyncApp, out_dir: PathBuf) {
    let _ = std::fs::create_dir_all(&out_dir);
    for (seq, step) in SCRIPT.iter().enumerate() {
        match step {
            TourStep::Nav(path) => {
                let path = path.to_string();
                cx.update(|_cx: &mut App| {
                    // The tour navigates through the same Navigator API a
                    // Link click uses; the router is the thread-current one.
                    Navigator::new().push(path);
                });
            }
            TourStep::WaitMs(ms) => {
                let ms = *ms;
                cx.background_executor()
                    .timer(Duration::from_millis(ms))
                    .await;
            }
            TourStep::Shot(name) => {
                let path = out_dir.join(format!("{seq:02}_{name}.png"));
                eprintln!("TOUR: capturing {}", path.display());
                capture_frame(&path);
            }
            TourStep::DevtoolsTab(name) => {
                let tab = tab_from_name(name);
                cx.update(|cx: &mut App| {
                    if let Some(tour) = cx.try_global::<TourGlobal>() {
                        tour.devtools.borrow_mut().tour_set_tab(tab);
                    }
                });
            }
            TourStep::DevtoolsSearch(query) => {
                let query = query.to_string();
                cx.update(|cx: &mut App| {
                    if let Some(tour) = cx.try_global::<TourGlobal>() {
                        tour.devtools.borrow_mut().tour_set_timeline_search(&query);
                    }
                });
            }
            TourStep::DevtoolsToggle => {
                cx.update(|cx: &mut App| {
                    if let Some(tour) = cx.try_global::<TourGlobal>() {
                        tour.devtools.borrow_mut().tour_toggle_expanded();
                    }
                });
            }
        }
    }
    eprintln!("TOUR: complete");
    std::process::exit(0);
}
