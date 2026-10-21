// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // WebKitGTK's DMABUF renderer crashes with Wayland "Error 71" on NVIDIA's
  // proprietary driver (explicit sync, WebKit bug 280210). Disabling explicit
  // sync keeps GPU rendering without triggering the protocol error.
  #[cfg(target_os = "linux")]
  if std::path::Path::new("/sys/module/nvidia").exists()
    && std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none()
  {
    std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
  }

  artcraft_app_lib::run();
}
