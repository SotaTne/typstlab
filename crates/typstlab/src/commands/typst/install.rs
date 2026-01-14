//! Typst install command - download and install Typst from GitHub Releases

use anyhow::Result;
use std::path::PathBuf;
use typstlab_core::{TypstlabError, project::Project};
use typstlab_typst::install::{
    DownloadOptions, download_and_install, fetch_release_metadata,
    select_asset_for_current_platform,
};

use super::link;

/// Execute `typstlab typst install` command
pub fn execute_install(version: String, from_cargo: bool) -> Result<()> {
    // Find project root
    let project = Project::from_current_dir()?;
    let root = &project.root;

    if from_cargo {
        // TODO(v0.2): Implement cargo install fallback
        // Per DESIGN.md, cargo should be automatic fallback when GitHub fails
        eprintln!("✗ --from-cargo flag not yet implemented");
        eprintln!("Automatic cargo fallback will be available in v0.2");
        eprintln!("Currently only GitHub Releases installation is supported");
        return Err(
            TypstlabError::Generic("cargo installation not yet implemented".to_string()).into(),
        );
    }

    println!("Installing Typst {} from GitHub Releases...", version);

    // 1. Fetch release metadata
    let tag = format!("v{}", version);
    println!("Fetching release metadata for {}...", tag);
    let release = fetch_release_metadata(&tag)?;

    // 2. Select asset for current platform
    println!("Selecting asset for current platform...");
    let asset = select_asset_for_current_platform(&release)?;
    println!("Selected asset: {}", asset.name);

    // 3. Determine cache directory
    let cache_dir = determine_cache_dir()?;
    println!("Cache directory: {}", cache_dir.display());

    // 4. Download and install
    println!("Downloading and installing...");
    let options = DownloadOptions {
        cache_dir: cache_dir.clone(),
        version: version.clone(),
        progress: Some(progress_callback),
    };

    let binary_path = download_and_install(asset, options)?;
    println!("✓ Installed to: {}", binary_path.display());

    // 5. Verify installation by running typst --version
    println!("Verifying installation...");
    let output = std::process::Command::new(&binary_path)
        .arg("--version")
        .output()
        .map_err(|e| TypstlabError::Generic(format!("Failed to verify installation: {}", e)))?;

    if !output.status.success() {
        return Err(TypstlabError::Generic(format!(
            "Installation verification failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }

    let installed_version = String::from_utf8_lossy(&output.stdout);
    let installed_version_num = installed_version
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| TypstlabError::Generic("Failed to parse installed version".to_string()))?;

    if installed_version_num != version {
        return Err(TypstlabError::Generic(format!(
            "Version mismatch: expected {}, got {}",
            version, installed_version_num
        ))
        .into());
    }

    println!("✓ Verified: Typst {}", installed_version_num);

    // 6. Create bin/typst shim (reuse from link.rs)
    link::create_bin_shim(root, &binary_path)?;

    // 7. Update state.json (reuse from link.rs)
    link::update_state(root, &binary_path, &version, "managed".to_string())?;

    println!("✓ Typst {} installation complete", version);

    Ok(())
}

/// Determine managed cache directory (per DESIGN.md 6.1.2)
/// Uses OS default cache locations only, does not respect environment variables
fn determine_cache_dir() -> Result<PathBuf> {
    // Per DESIGN.md 6.1.2: Use OS default cache locations only
    // Do NOT respect XDG_CACHE_HOME or other environment variables
    let cache_base = {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .ok_or_else(|| {
                    TypstlabError::Generic("Could not determine home directory".to_string())
                })?
                .join("Library")
                .join("Caches")
        }
        #[cfg(target_os = "linux")]
        {
            dirs::home_dir()
                .ok_or_else(|| {
                    TypstlabError::Generic("Could not determine home directory".to_string())
                })?
                .join(".cache")
        }
        #[cfg(target_os = "windows")]
        {
            dirs::cache_dir().ok_or_else(|| {
                TypstlabError::Generic("Could not determine cache directory".to_string())
            })?
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            return Err(TypstlabError::Generic("Unsupported platform".to_string()).into());
        }
    };

    let typstlab_cache = cache_base.join("typstlab").join("typst");
    Ok(typstlab_cache)
}

/// Progress callback for download
fn progress_callback(downloaded: u64, total: u64) {
    if total > 0 {
        let percent = (downloaded as f64 / total as f64 * 100.0) as u8;
        let mb_downloaded = downloaded as f64 / 1_048_576.0;
        let mb_total = total as f64 / 1_048_576.0;
        print!(
            "\r  Progress: {} / {} MB ({}%)",
            mb_downloaded, mb_total, percent
        );
        std::io::Write::flush(&mut std::io::stdout()).ok();

        if downloaded == total {
            println!(); // Newline after completion
        }
    }
}
