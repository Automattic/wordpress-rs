use anyhow::{Context, Result};
use clap::*;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

const XCFRAMEWORK_OUTPUT_PATH: &str = "target/libwordpressFFI.xcframework";
const SWIFT_BINDINGS_HEADER_DIR: &str = "target/swift-bindings/headers";
const LIBRARY_FILENAME: &str = "libwordpress.a";

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
        let temp_dir = std::env::temp_dir().join("wp-rs-xcframework");
        recreate_directory(&temp_dir)?;

        XCFramework::new(&self.targets, &self.profile)?.create(&temp_dir)?;

        std::fs::remove_dir_all(&temp_dir)
            .with_context(|| format!("Failed to remove temporary dir: {:?}", temp_dir))
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
        let headers = PathBuf::from(SWIFT_BINDINGS_HEADER_DIR);
        if !headers.exists() {
            anyhow::bail!("Headers not found: {}", headers.display())
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
        self.patch_xcframework(&temp_dest)?;

        let dest = PathBuf::from(XCFRAMEWORK_OUTPUT_PATH);
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
        self.libraries
            .iter()
            .map(|lib| lib.create(temp_dir))
            .collect()
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

    // Fixes an issue including the XCFramework in an Xcode project that already contains an XCFramework: https://github.com/jessegrosjean/module-map-error
    fn patch_xcframework(&self, temp_dir: &Path) -> Result<()> {
        println!("Patching XCFramework to have a unique header directory");

        for dir_entry in std::fs::read_dir(temp_dir)? {
            let path = dir_entry.expect("Invalid Path").path();
            if path.is_dir() {
                let headers_dir = temp_dir.join(&path).join("Headers");
                let header_files: Vec<PathBuf> = std::fs::read_dir(&headers_dir)?
                    .flat_map(|f| f.ok())
                    .map(|f| f.path())
                    .filter(|f| f.extension() != Some(std::ffi::OsStr::new("a")))
                    .collect();

                let new_headers_dir = headers_dir.join("libwordpressFFI");
                recreate_directory(&new_headers_dir)?;

                for file in header_files {
                    std::fs::rename(&file, new_headers_dir.join(file.file_name().unwrap()))?;
                }
            }
        }

        Ok(())
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
        Command::new("xcrun")
            .arg("lipo")
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

        let lib = temp_dir.join("temp.a");
        Command::new("xcrun")
            .arg("libtool")
            .arg("-static")
            .arg("-o")
            .arg(&lib)
            .args(libs)
            .successful_output()?;

        if !lib.exists() {
            anyhow::bail!("Library not found: {}", lib.display())
        }

        let dir = temp_dir.join(&self.target);
        recreate_directory(&dir)?;

        let dest = dir.join(LIBRARY_FILENAME);
        std::fs::copy(&lib, &dest)
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

        vec![
            target_dir.join("libwp_api.a"),
            target_dir.join("libjetpack_api.a"),
        ]
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

impl TryFrom<&str> for ApplePlatform {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> std::result::Result<Self, anyhow::Error> {
        match s {
            "darwin" => Ok(ApplePlatform::MacOS),
            "ios" => Ok(ApplePlatform::IOS),
            "tvos" => Ok(ApplePlatform::TvOS),
            "watchos" => Ok(ApplePlatform::WatchOS),
            _ => anyhow::bail!("Unknown Apple platform: {}", s),
        }
    }
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
            anyhow::bail!("{} is not an Apple platform", target)
        }

        let os: ApplePlatform = parts
            .next()
            .with_context(|| format!("No OS in target: {}", target))?
            .try_into()?;

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
            .with_context(|| "No llvm-target in command output")?;

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
            anyhow::bail!(
                "Command failed with exit code: {}\nstdout: {:?}\nstderr: {:?}\n$ {:?}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                self
            )
        }
    }
}

fn recreate_directory(dir: &PathBuf) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .with_context(|| format!("Failed to remove directory at {:?}", dir))?;
    }

    std::fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {:?}", dir))
}
