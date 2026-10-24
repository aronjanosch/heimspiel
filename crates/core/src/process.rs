//! Child processes started in a process group of their own, so they can be stopped together with
//! everything they started.

use std::process::{Child, Command};

/// Sends SIGTERM to the child's process group and reaps the child.
pub(crate) fn stop(child: &mut Child) {
    let group = format!("-{}", child.id());
    let killed = Command::new("kill").args(["-TERM", "--", &group]).status();
    if !killed.is_ok_and(|s| s.success()) {
        let _ = child.kill();
    }
    let _ = child.wait();
}
