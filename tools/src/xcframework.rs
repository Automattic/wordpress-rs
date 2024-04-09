use anyhow::{Context, Error, Result};
use clap::*;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Action;

static LIBRARY_FILENAME: &str = "libwordpress.a";

#[derive(Debug, Args)]
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

impl Action for CreateXCFramework {
    fn run(&self) -> Result<()> {
        let temp_dir = std::env::temp_dir().join("wp-rs-xcframework");
        recreate_directory(&temp_dir)?;

        XCFramework::new(&self.targets, &self.profile)?.create(&temp_dir)?;

        Ok(())
    }
}

// Represent a xcframework that contains static libraries for multiple platforms.
//
// Since `xcodebuild -create-xcframework` command requires its `-libraray` not
// having duplicated platform. This type along with `LibraryGroup` and `Slice`
// work together to make it easier to create a xcframework.
struct XCFramework {
    libraries: Vec<LibraryGroup>,
    headers: PathBuf,
}

// Represent a group of static libraries that are built for the same platform.
struct LibraryGroup {
    id: LibraryGroupId,
    slices: Vec<Slice>,
}

// Represent a thin static library which is built with `cargo build --target <target> --profile <profile>`
struct Slice {
    target: String,
    profile: String,
}

impl XCFramework {
    fn new(targets: &Vec<String>, profile: &str) -> Result<Self> {
        let headers = PathBuf::from("target/swift-bindings/headers");
        if !headers.exists() {
            return Err(Error::msg(format!(
                "Headers not found: {}",
                headers.display()
            )));
        }

        let mut groups = HashMap::<LibraryGroupId, LibraryGroup>::new();
        for target in targets {
            let id = LibraryGroupId::from_target(target)?;
            let id_clone = id.clone();
            groups
                .entry(id)
                .or_insert(LibraryGroup {
                    id: id_clone,
                    slices: Vec::new(),
                })
                .slices
                .push(Slice {
                    target: target.clone(),
                    profile: profile.to_owned(),
                });
        }

        Ok(Self {
            libraries: groups.into_values().collect(),
            headers,
        })
    }

    fn create(&self, temp_dir: &Path) -> Result<PathBuf> {
        self.preview();

        let libraries = self.combine_libraries(temp_dir)?;
        let temp_dest = self.create_xcframework(&libraries, temp_dir)?;

        let dest = PathBuf::from("target/libwordpressFFI.xcframework");
        recreate_directory(&dest)?;
        std::fs::rename(temp_dest, &dest).with_context(|| "Failed to move xcframework")?;

        println!("xcframework created at {}", &dest.display());
        Ok(dest)
    }

    fn preview(&self) {
        println!("Creating xcframework to include the following targets:");
        for lib in &self.libraries {
            println!("  Platform: {}", lib.id);
            for slice in &lib.slices {
                println!("    - {}", slice.target);
            }
        }
    }

    fn combine_libraries(&self, temp_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut libraries: Vec<PathBuf> = Vec::new();
        for lib in &self.libraries {
            libraries.push(lib.create(temp_dir)?);
        }
        Ok(libraries)
    }

    fn create_xcframework(&self, libraries: &[PathBuf], temp_dir: &Path) -> Result<PathBuf> {
        let temp_dest = temp_dir.join("libwordpressFFI.xcframework");
        std::fs::remove_dir_all(&temp_dest).ok();

        let library_args = libraries.iter().flat_map(|lib| {
            [
                "-library".as_ref(),
                lib.as_os_str(),
                "-headers".as_ref(),
                self.headers.as_os_str(),
            ]
        });
        Command::new("xcodebuild")
            .arg("-create-xcframework")
            .args(library_args)
            .arg("-output")
            .arg(&temp_dest)
            .successful_output()?;

        Ok(temp_dest)
    }
}

impl LibraryGroup {
    fn create(&self, temp_dir: &Path) -> Result<PathBuf> {
        let mut libraries: Vec<PathBuf> = Vec::new();
        for slice in &self.slices {
            libraries.push(slice.create(temp_dir)?);
        }

        let dir = temp_dir.join(self.id.to_string());
        recreate_directory(&dir)?;

        let dest = dir.join(LIBRARY_FILENAME);
        Command::new("lipo")
            .arg("-create")
            .args(libraries)
            .arg("-output")
            .arg(&dest)
            .successful_output()?;

        Ok(dest)
    }
}

impl Slice {
    fn create(&self, temp_dir: &Path) -> Result<PathBuf> {
        let libs = self.built_libraries();

        // If there are more static libraries (a.k.a cargo packages), we'll
        // need to bundle them together into one static library.
        // At the moment, we only have one libwp_api, so we can just copy it.
        assert!(
            libs.len() == 1,
            "Expected exactly one library for each slice"
        );

        let lib = &libs[0];
        if !lib.exists() {
            return Err(Error::msg(format!("Library not found: {}", lib.display())));
        }

        let dir = temp_dir.join(&self.target);
        recreate_directory(&dir)?;

        let dest = dir.join(LIBRARY_FILENAME);
        std::fs::copy(lib, &dest)
            .with_context(|| format!("Failed to copy {} to {}", lib.display(), dest.display()))?;

        Ok(dest)
    }

    fn built_libraries(&self) -> Vec<PathBuf> {
        let mut target_dir: PathBuf = ["target", &self.target].iter().collect();
        if self.profile == "dev" {
            target_dir.push("debug");
        } else {
            target_dir.push(&self.profile);
        }

        vec![target_dir.join("libwp_api.a")]
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct LibraryGroupId {
    os: ApplePlatform,
    is_sim: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum ApplePlatform {
    MacOS,
    #[allow(clippy::upper_case_acronyms)]
    IOS,
    TvOS,
    WatchOS,
}

impl Display for ApplePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ApplePlatform::MacOS => "macos",
            ApplePlatform::IOS => "ios",
            ApplePlatform::TvOS => "tvos",
            ApplePlatform::WatchOS => "watchos",
        };
        write!(f, "{}", name)
    }
}

impl LibraryGroupId {
    fn from_target(target: &str) -> Result<Self> {
        let mut parts = target.split('-');
        _ /* arch */= parts.next();
        if parts.next() != Some("apple") {
            return Err(Error::msg(format!("{} is not an Apple platform", target)));
        }

        let os = match parts.next() {
            Some("darwin") => ApplePlatform::MacOS,
            Some("ios") => ApplePlatform::IOS,
            Some("tvos") => ApplePlatform::TvOS,
            Some("watchos") => ApplePlatform::WatchOS,
            _ => return Err(Error::msg(format!("Unknown OS in target: {}", target))),
        };

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
            .successful_output()?;
        let json = serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .with_context(|| "Failed to parse command output as JSON")?;
        let llvm_target = json
            .get("llvm-target")
            .and_then(|t| t.as_str())
            .ok_or(Error::msg("No llvm-target in command output"))?;

        Ok(Self {
            os,
            is_sim: llvm_target.ends_with("-simulator"),
        })
    }
}

impl Display for LibraryGroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.os)?;

        if self.is_sim {
            write!(f, "-sim")
        } else {
            Ok(())
        }
    }
}

trait ExecuteCommand {
    fn successful_output(&mut self) -> Result<std::process::Output>;
}

impl ExecuteCommand for Command {
    fn successful_output(&mut self) -> Result<std::process::Output> {
        let output = self
            .output()
            .with_context(|| format!("Command failed: $ {:?}", self))?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::msg(format!(
                "Command failed with exit code: {}\n$ {:?}",
                output.status, self
            )))
        }
    }
}

fn recreate_directory(dir: &PathBuf) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .with_context(|| format!("Failed to remove directory at {:?}", dir))?;
    }

    std::fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create directory: {:?}", dir))?;

    Ok(())
}
