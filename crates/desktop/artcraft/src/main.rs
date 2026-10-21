// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
const NVIDIA_MODULE_PATH: &str = "/sys/module/nvidia";
#[cfg(target_os = "linux")]
const NV_DISABLE_EXPLICIT_SYNC_ENV: &str = "__NV_DISABLE_EXPLICIT_SYNC";

fn main() {
  #[cfg(target_os = "linux")]
  configure_linux_nvidia_env(Path::new(NVIDIA_MODULE_PATH));

  artcraft_app_lib::run();
}

// WebKitGTK's DMABUF renderer crashes with Wayland "Error 71" on NVIDIA's
// proprietary driver (explicit sync, WebKit bug 280210). Disabling explicit
// sync keeps GPU rendering without triggering the protocol error.
#[cfg(target_os = "linux")]
fn configure_linux_nvidia_env(nvidia_module_path: &Path) {
  if nvidia_module_path.exists() && std::env::var_os(NV_DISABLE_EXPLICIT_SYNC_ENV).is_none() {
    std::env::set_var(NV_DISABLE_EXPLICIT_SYNC_ENV, "1");
  }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
  use super::*;
  use serial_test::serial;

  mod configure_linux_nvidia_env_tests {
    use super::*;

    #[test]
    #[serial]
    fn sets_env_when_nvidia_present_and_unset() {
      std::env::remove_var(NV_DISABLE_EXPLICIT_SYNC_ENV);
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");

      configure_linux_nvidia_env(temp_dir.path());

      assert_eq!(
        std::env::var(NV_DISABLE_EXPLICIT_SYNC_ENV).as_deref(),
        Ok("1")
      );
      std::env::remove_var(NV_DISABLE_EXPLICIT_SYNC_ENV);
    }

    #[test]
    #[serial]
    fn preserves_existing_user_override() {
      std::env::set_var(NV_DISABLE_EXPLICIT_SYNC_ENV, "0");
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");

      configure_linux_nvidia_env(temp_dir.path());

      assert_eq!(
        std::env::var(NV_DISABLE_EXPLICIT_SYNC_ENV).as_deref(),
        Ok("0")
      );
      std::env::remove_var(NV_DISABLE_EXPLICIT_SYNC_ENV);
    }

    #[test]
    #[serial]
    fn skips_when_nvidia_module_absent() {
      std::env::remove_var(NV_DISABLE_EXPLICIT_SYNC_ENV);
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");
      let missing_path = temp_dir.path().join("nonexistent_nvidia");

      configure_linux_nvidia_env(&missing_path);

      assert!(std::env::var_os(NV_DISABLE_EXPLICIT_SYNC_ENV).is_none());
    }
  }
}
