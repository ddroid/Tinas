use std::path::{Path, PathBuf};
use anyhow::Result;

/// Information about a detected browser installation
#[derive(Debug, Clone)]
pub struct BrowserInfo {
    pub name: String,
    pub path: PathBuf,
    pub pak_files: Vec<PathBuf>,
}

/// Detects all available browser installations with their pak files
pub fn detect_browsers() -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();

    // Check common Brave installation paths
    let brave_paths = get_brave_install_paths();

    for brave_path in brave_paths {
        if brave_path.exists() {
            println!("Found Brave at: {:?}", brave_path);
            if let Ok(pak_files) = scan_for_pak_files(&brave_path) {
                if !pak_files.is_empty() {
                    browsers.push(BrowserInfo {
                        name: format!("Brave ({})", brave_path.display()),
                        path: brave_path,
                        pak_files,
                    });
                }
            }
        }
    }

    browsers
}

/// Detects Brave browser installation and returns paths to all .pak files
pub fn detect_brave_pak_files() -> Vec<PathBuf> {
    detect_browsers()
        .into_iter()
        .flat_map(|b| b.pak_files)
        .collect()
}

/// Get possible Brave installation paths based on OS
fn get_brave_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // AUR/ Manual installation in /opt
        paths.push(PathBuf::from("/opt/brave-bin"));
        // Flatpak installation
        paths.push(PathBuf::from(
            "/var/lib/flatpak/app/com.brave.Browser/current/active/files/brave"
        ));
        // User Flatpak
        paths.push(
            dirs::home_dir()
                .map(|h| h.join(".var/app/com.brave.Browser/config/brave"))
                .unwrap_or_default(),
        );
        // System installation
        paths.push(PathBuf::from("/usr/lib/brave-browser"));
        paths.push(PathBuf::from("/usr/lib/brave"));
        // Snap
        paths.push(PathBuf::from("/snap/brave/current/usr/lib/brave-browser"));
        // Additional common paths
        paths.push(PathBuf::from("/opt/brave"));
        paths.push(PathBuf::from("/usr/share/brave"));
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(
            dirs::home_dir()
                .map(|h| h.join("Applications/Brave Browser.app/Contents/Frameworks"))
                .unwrap_or_default(),
        );
        paths.push(PathBuf::from(
            "/Applications/Brave Browser.app/Contents/Frameworks"
        ));
    }

    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from("C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application"));
        paths.push(PathBuf::from(
            "C:\\Program Files (x86)\\BraveSoftware\\Brave-Browser\\Application",
        ));
        // User install
        if let Some(local_app_data) = dirs::data_local_dir() {
            paths.push(local_app_data.join("BraveSoftware\\Brave-Browser\\Application"));
        }
    }

    paths
}

/// Recursively scan directory for .pak files
fn scan_for_pak_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut pak_files = Vec::new();

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "pak" {
                        println!("Found pak file: {:?}", path);
                        pak_files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories (limit depth by not going too deep)
                if let Ok(mut sub_files) = scan_for_pak_files(&path) {
                    pak_files.append(&mut sub_files);
                }
            }
        }
    }

    Ok(pak_files)
}

/// Get a user-friendly name for a pak file path
pub fn get_pak_display_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pak_display_name() {
        let path = PathBuf::from("/some/path/chrome_100_percent.pak");
        assert_eq!(get_pak_display_name(&path), "chrome_100_percent");
    }
}
