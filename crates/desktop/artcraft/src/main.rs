// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
const NVIDIA_MODULE_PATH: &str = "/sys/module/nvidia";
#[cfg(target_os = "linux")]
const NV_DISABLE_EXPLICIT_SYNC_ENV: &str = "__NV_DISABLE_EXPLICIT_SYNC";
#[cfg(target_os = "linux")]
const APPDIR_ENV: &str = "APPDIR";
#[cfg(target_os = "linux")]
const GST_PLUGIN_PATH_ENV: &str = "GST_PLUGIN_PATH_1_0";
#[cfg(target_os = "linux")]
const GST_PLUGIN_SCANNER_ENV: &str = "GST_PLUGIN_SCANNER_1_0";
#[cfg(target_os = "linux")]
const SYSTEM_GST_PLUGIN_DIRS: &[&str] = &[
  "/usr/lib/gstreamer-1.0",
  "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
  "/usr/lib/aarch64-linux-gnu/gstreamer-1.0",
  "/usr/lib64/gstreamer-1.0",
];
#[cfg(target_os = "linux")]
const SYSTEM_GST_SCANNER_PATHS: &[&str] = &[
  "/usr/lib/gstreamer-1.0/gst-plugin-scanner",
  "/usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner",
  "/usr/lib/aarch64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner",
  "/usr/libexec/gstreamer-1.0/gst-plugin-scanner",
  "/usr/lib64/gstreamer-1.0/gst-plugin-scanner",
];

fn main() {
  #[cfg(target_os = "linux")]
  {
    configure_linux_nvidia_env(Path::new(NVIDIA_MODULE_PATH));
    if let Some(appdir) = std::env::var_os(APPDIR_ENV) {
      configure_linux_appimage_gstreamer_env(
        Path::new(&appdir),
        SYSTEM_GST_PLUGIN_DIRS,
        SYSTEM_GST_SCANNER_PATHS,
      );
    }
  }

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

// AppImage bundles libgstreamer without usr/lib/gstreamer-1.0 plugins (#2533),
// causing bundled GStreamer to miss host plugins and abort WebKitWebProcess.
#[cfg(target_os = "linux")]
fn configure_linux_appimage_gstreamer_env(
  appdir: &Path,
  plugin_dir_candidates: &[&str],
  scanner_candidates: &[&str],
) {
  if appdir.join("usr/lib/gstreamer-1.0").exists() {
    return;
  }

  if std::env::var_os(GST_PLUGIN_PATH_ENV).is_none() {
    let existing_dirs: Vec<&str> = plugin_dir_candidates
      .iter()
      .copied()
      .filter(|dir| Path::new(dir).is_dir())
      .collect();
    if !existing_dirs.is_empty() {
      std::env::set_var(GST_PLUGIN_PATH_ENV, existing_dirs.join(":"));
    }
  }

  if std::env::var_os(GST_PLUGIN_SCANNER_ENV).is_none() {
    if let Some(scanner) = scanner_candidates
      .iter()
      .copied()
      .find(|path| Path::new(path).is_file())
    {
      std::env::set_var(GST_PLUGIN_SCANNER_ENV, scanner);
    }
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

  mod configure_linux_appimage_gstreamer_env_tests {
    use super::*;

    #[test]
    #[serial]
    fn sets_host_plugin_and_scanner_paths_when_appimage_lacks_plugins() {
      clear_gst_env();
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");
      let appdir = temp_dir.path().join("appdir");
      let host_plugins = temp_dir.path().join("host_plugins");
      let host_scanner = temp_dir.path().join("gst-plugin-scanner");
      std::fs::create_dir_all(&appdir).expect("appdir should be created");
      std::fs::create_dir_all(&host_plugins).expect("host_plugins should be created");
      std::fs::write(&host_scanner, b"").expect("host_scanner should be created");

      let plugin_str = host_plugins.to_str().unwrap();
      let scanner_str = host_scanner.to_str().unwrap();

      configure_linux_appimage_gstreamer_env(&appdir, &[plugin_str], &[scanner_str]);

      assert_eq!(std::env::var(GST_PLUGIN_PATH_ENV).as_deref(), Ok(plugin_str));
      assert_eq!(
        std::env::var(GST_PLUGIN_SCANNER_ENV).as_deref(),
        Ok(scanner_str)
      );
      clear_gst_env();
    }

    #[test]
    #[serial]
    fn skips_when_appimage_bundles_gstreamer_plugins() {
      clear_gst_env();
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");
      let appdir = temp_dir.path().join("appdir");
      std::fs::create_dir_all(appdir.join("usr/lib/gstreamer-1.0"))
        .expect("bundled plugin dir should be created");
      let host_plugins = temp_dir.path().join("host_plugins");
      std::fs::create_dir_all(&host_plugins).expect("host_plugins should be created");
      let plugin_str = host_plugins.to_str().unwrap();

      configure_linux_appimage_gstreamer_env(&appdir, &[plugin_str], &[]);

      assert!(std::env::var_os(GST_PLUGIN_PATH_ENV).is_none());
      clear_gst_env();
    }

    #[test]
    #[serial]
    fn preserves_existing_gst_env_overrides() {
      clear_gst_env();
      std::env::set_var(GST_PLUGIN_PATH_ENV, "/custom/plugins");
      std::env::set_var(GST_PLUGIN_SCANNER_ENV, "/custom/scanner");
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");
      let host_plugins = temp_dir.path().join("host_plugins");
      let host_scanner = temp_dir.path().join("gst-plugin-scanner");
      std::fs::create_dir_all(&host_plugins).expect("host_plugins should be created");
      std::fs::write(&host_scanner, b"").expect("host_scanner should be created");

      configure_linux_appimage_gstreamer_env(
        temp_dir.path(),
        &[host_plugins.to_str().unwrap()],
        &[host_scanner.to_str().unwrap()],
      );

      assert_eq!(
        std::env::var(GST_PLUGIN_PATH_ENV).as_deref(),
        Ok("/custom/plugins")
      );
      assert_eq!(
        std::env::var(GST_PLUGIN_SCANNER_ENV).as_deref(),
        Ok("/custom/scanner")
      );
      clear_gst_env();
    }

    #[test]
    #[serial]
    fn discovers_real_host_gstreamer_elements_under_appimage() {
      clear_gst_env();
      let temp_dir = tempfile::tempdir().expect("tempdir should be created");
      let appdir = temp_dir.path().join("appdir");
      std::fs::create_dir_all(&appdir).expect("appdir should be created");
      let missing_appdir_plugins = appdir.join("usr/lib/gstreamer-1.0");
      let registry_before = temp_dir.path().join("registry_before.bin");
      let registry_after = temp_dir.path().join("registry_after.bin");

      if Path::new("/usr/bin/gst-inspect-1.0").is_file() {
        let broken_status = std::process::Command::new("/usr/bin/gst-inspect-1.0")
          .arg("appsink")
          .env("GST_PLUGIN_SYSTEM_PATH_1_0", &missing_appdir_plugins)
          .env_remove(GST_PLUGIN_PATH_ENV)
          .env("GST_REGISTRY_1_0", &registry_before)
          .stdout(std::process::Stdio::null())
          .stderr(std::process::Stdio::null())
          .status()
          .expect("gst-inspect-1.0 should execute");
        assert!(
          !broken_status.success(),
          "appsink should fail when AppImage lacks plugins and GST_PLUGIN_PATH_1_0 is unset"
        );
      }

      configure_linux_appimage_gstreamer_env(
        &appdir,
        SYSTEM_GST_PLUGIN_DIRS,
        SYSTEM_GST_SCANNER_PATHS,
      );

      let plugin_path = std::env::var(GST_PLUGIN_PATH_ENV).expect("host plugin path should be set");
      let scanner_path =
        std::env::var(GST_PLUGIN_SCANNER_ENV).expect("host scanner path should be set");
      assert!(Path::new(&scanner_path).is_file());

      if Path::new("/usr/bin/gst-inspect-1.0").is_file() {
        for element in [
          "appsrc",
          "appsink",
          "autoaudiosink",
          "audioconvert",
          "audioresample",
        ] {
          let status = std::process::Command::new("/usr/bin/gst-inspect-1.0")
            .arg(element)
            .env("GST_PLUGIN_SYSTEM_PATH_1_0", &missing_appdir_plugins)
            .env(GST_PLUGIN_PATH_ENV, &plugin_path)
            .env(GST_PLUGIN_SCANNER_ENV, &scanner_path)
            .env("GST_REGISTRY_1_0", &registry_after)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("gst-inspect-1.0 should execute");
          assert!(status.success(), "GStreamer element {element} should be found");
        }
      }

      clear_gst_env();
    }

    fn clear_gst_env() {
      std::env::remove_var(GST_PLUGIN_PATH_ENV);
      std::env::remove_var(GST_PLUGIN_SCANNER_ENV);
    }
  }
}
