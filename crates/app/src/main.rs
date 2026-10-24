fn main() {
    work_around_nvidia_wayland();
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the heimspiel app");
}

/// WebKitGTK does not start on NVIDIA under Wayland (`Error 71 (Protocol error)`), see ADR 0002.
/// Must run before GTK initialises and before any other thread exists.
fn work_around_nvidia_wayland() {
    #[cfg(target_os = "linux")]
    {
        use std::{env, path::Path};

        let wayland = env::var_os("WAYLAND_DISPLAY").is_some();
        let nvidia = Path::new("/proc/driver/nvidia/version").exists();
        let user_set = env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_some()
            || env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some();
        if wayland && nvidia && !user_set {
            // SAFETY: called first thing in `main`, while the process is still single-threaded.
            unsafe { env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1") };
        }
    }
}
