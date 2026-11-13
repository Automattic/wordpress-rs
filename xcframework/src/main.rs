//! Assembles `target/libwordpressFFI.xcframework` from the per-target static libraries and Swift
//! binding headers that `make build-apple-*` leaves in `target/<target>/<profile>/`.
//!
//! The xcframework is written directly rather than with `xcodebuild -create-xcframework`, so it
//! can be assembled on Linux as well as macOS; `xcframework.rs` mirrors what `xcodebuild` produces.
//! Fat libraries come from `lipo` on macOS and LLVM's `llvm-lipo` elsewhere, which write identical
//! files. Set `LIPO` to use a different tool.

use anyhow::{Context, Result};
use clap::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

mod xcframework;

use xcframework::{Arch, Library, Platform, Variant};

const XCFRAMEWORK_OUTPUT_PATH: &str = "target/libwordpressFFI.xcframework";
const LIBRARY_FILENAME: &str = "libwp_mobile.a";
// Headers go in a uniquely named subdirectory so the module map doesn't collide with other
// xcframeworks in the same Xcode project: https://github.com/jessegrosjean/module-map-error
const HEADERS_SUBDIRECTORY: &str = "libwordpressFFI";

fn main() -> Result<()> {
    CreateXCFramework::parse().run()
}

#[derive(Debug, Parser)]
pub struct CreateXCFramework {
    // Non-empty list of targets
    #[clap(
        long,
        num_args = 1..,
        required = true,
        help = "List of targets whose static libraries should be included in the xcframework"
    )]
    targets: Vec<String>,

    #[clap(
        long,
        default_value = "release",
        help = "Cargo profile used to build the targets"
    )]
    profile: String,
}

impl CreateXCFramework {
    fn run(&self) -> Result<()> {
        let mut libraries = BTreeMap::<(Platform, Variant), Vec<Slice>>::new();
        for target in &self.targets {
            let slice = Slice::new(target, &self.profile)?;
            libraries
                .entry((slice.platform, slice.variant))
                .or_default()
                .push(slice);
        }

        println!("Creating xcframework to include the following targets:");
        for ((platform, variant), slices) in &libraries {
            println!("  Platform: {platform}{}", variant.suffix());
            for slice in slices {
                println!("    - {}", slice.target);
            }
        }

        let temp_dest = std::env::temp_dir()
            .join("wp-rs-xcframework")
            .join("libwordpressFFI.xcframework");
        recreate_directory(&temp_dest)?;

        let mut entries = Vec::new();
        for ((platform, variant), slices) in libraries {
            entries.push(write_library(&temp_dest, platform, variant, &slices)?);
        }
        xcframework::write_info_plist(&temp_dest, &entries)?;

        let dest = PathBuf::from(XCFRAMEWORK_OUTPUT_PATH);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .with_context(|| format!("Failed to remove {}", dest.display()))?;
        }
        move_dir(&temp_dest, &dest)?;

        println!("xcframework created at {}", dest.display());
        Ok(())
    }
}

/// Writes one `<library identifier>/` directory: the (possibly fat) static library and its headers.
fn write_library(
    xcframework: &Path,
    platform: Platform,
    variant: Variant,
    slices: &[Slice],
) -> Result<Library> {
    let mut archs: Vec<Arch> = slices.iter().map(|s| s.arch).collect();
    archs.sort();
    if let Some(pair) = archs.windows(2).find(|pair| pair[0] == pair[1]) {
        anyhow::bail!(
            "Two targets for {platform}{} have the same architecture ({})",
            variant.suffix(),
            pair[0]
        );
    }

    let library = Library {
        platform,
        variant,
        archs,
        binary: LIBRARY_FILENAME.to_string(),
        headers: "Headers".to_string(),
    };
    let dir = xcframework.join(library.identifier());
    std::fs::create_dir_all(&dir)?;

    // Single-architecture libraries are wrapped in a one-slice fat file too, as the
    // `xcodebuild`-based tool did, so the output is unchanged.
    let dest = dir.join(LIBRARY_FILENAME);
    let inputs = slices
        .iter()
        .map(Slice::library)
        .collect::<Result<Vec<_>>>()?;
    lipo_create(&inputs, &dest)?;

    // Every slice of a library shares one set of headers; use the first.
    let headers_src = slices[0].headers_dir()?;
    let headers_dest = dir.join(&library.headers).join(HEADERS_SUBDIRECTORY);
    std::fs::create_dir_all(&headers_dest)?;
    for entry in std::fs::read_dir(&headers_src)? {
        let path = entry?.path();
        if path.is_file() {
            std::fs::copy(&path, headers_dest.join(path.file_name().unwrap()))?;
        }
    }

    Ok(library)
}

/// A thin static library built with `cargo rustc --target <target> --profile <profile>`.
struct Slice {
    target: String,
    profile: String,
    platform: Platform,
    variant: Variant,
    arch: Arch,
}

impl Slice {
    fn new(target: &str, profile: &str) -> Result<Self> {
        let llvm_target = llvm_target(target)?;
        let (platform, variant, arch) = xcframework::classify(target, &llvm_target)?;
        Ok(Self {
            target: target.to_string(),
            profile: profile.to_string(),
            platform,
            variant,
            arch,
        })
    }

    /// The directory where `cargo` puts this target's build products.
    fn built_product_dir(&self) -> PathBuf {
        let profile_dir = if self.profile == "dev" {
            "debug"
        } else {
            &self.profile
        };
        ["target", &self.target, profile_dir].iter().collect()
    }

    fn library(&self) -> Result<PathBuf> {
        let lib = self.built_product_dir().join(LIBRARY_FILENAME);
        if !lib.exists() {
            anyhow::bail!("Library not found: {}", lib.display())
        }
        Ok(lib)
    }

    // `scripts/swift-bindings.sh` writes `Headers`; the exact case matters on Linux.
    fn headers_dir(&self) -> Result<PathBuf> {
        let path = self.built_product_dir().join("swift-bindings/Headers");
        if !path.is_dir() {
            anyhow::bail!("Headers not found: {}", path.display())
        }
        Ok(path)
    }
}

/// Asks `rustc` for the LLVM triple of a target, which spells out simulator and Mac Catalyst
/// variants (e.g. `arm64-apple-ios16.0.0-simulator`).
fn llvm_target(target: &str) -> Result<String> {
    let output = Command::new("rustc")
        .env("RUSTC_BOOTSTRAP", "1")
        .args([
            "-Z",
            "unstable-options",
            "--print",
            "target-spec-json",
            "--target",
        ])
        .arg(target)
        .output()
        .context("Failed to run rustc")?;
    if !output.status.success() {
        anyhow::bail!(
            "rustc could not describe target {target}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .context("Failed to parse rustc target spec as JSON")?;
    json.get("llvm-target")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .context("No llvm-target in rustc target spec")
}

/// Combines thin static libraries into one fat library with `lipo -create`.
///
/// `llvm-lipo` reads the objects inside each library, so it must be at least as new as the LLVM
/// that rustc used to build them. An older one fails with "Unknown attribute kind".
fn lipo_create(inputs: &[PathBuf], dest: &Path) -> Result<()> {
    let default = if cfg!(target_os = "macos") {
        "lipo"
    } else {
        "llvm-lipo"
    };
    let lipo = std::env::var_os("LIPO").unwrap_or_else(|| default.into());
    let status = Command::new(&lipo)
        .arg("-create")
        .args(inputs)
        .arg("-output")
        .arg(dest)
        .status()
        .with_context(|| {
            format!(
                "Failed to run {}. Install LLVM, or set LIPO to a lipo-compatible tool.",
                lipo.to_string_lossy()
            )
        })?;
    if !status.success() {
        anyhow::bail!(
            "{} failed to create {}",
            lipo.to_string_lossy(),
            dest.display()
        );
    }
    Ok(())
}

fn recreate_directory(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .with_context(|| format!("Failed to remove directory at {dir:?}"))?;
    }
    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {dir:?}"))
}

/// Moves a directory, falling back to copy-and-delete when `src` and `dest` are on different
/// filesystems (common on Linux, where the temp dir is often a separate mount).
fn move_dir(src: &Path, dest: &Path) -> Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_dir(src, dest)?;
    std::fs::remove_dir_all(src).with_context(|| format!("Failed to remove {}", src.display()))
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)
                .with_context(|| format!("Failed to copy to {}", to.display()))?;
        }
    }
    Ok(())
}
