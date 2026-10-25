//! Child processes started in a process group of their own, so they can be stopped together with
//! everything they started.

use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

/// Sends SIGTERM to the child's process group, and SIGKILL when the child is still there after a
/// few seconds (gamescope ignores SIGTERM), then reaps the child.
pub(crate) fn stop(child: &mut Child) {
    let group = format!("-{}", child.id());
    let signal = |name: &str| {
        Command::new("kill")
            .args([name, "--", &group])
            .status()
            .is_ok_and(|s| s.success())
    };
    if !signal("-TERM") {
        let _ = child.kill();
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        if child.try_wait().ok().flatten().is_some() {
            // The child is gone; the rest of its group may still ignore SIGTERM.
            signal("-KILL");
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    signal("-KILL");
    let _ = child.kill();
    let _ = child.wait();
}
