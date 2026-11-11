//! The parts of an xcframework that `xcodebuild -create-xcframework` would otherwise write: library
//! identifiers and `Info.plist`. The output matches what `xcodebuild` produced for this project.

use anyhow::{Context, Result};
use std::fmt::{Display, Formatter};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    MacOS,
    #[allow(clippy::upper_case_acronyms)]
    IOS,
    TvOS,
    WatchOS,
}

impl Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Platform::MacOS => "macos",
            Platform::IOS => "ios",
            Platform::TvOS => "tvos",
            Platform::WatchOS => "watchos",
        })
    }
}

/// `SupportedPlatformVariant`. Mac Catalyst is a variant of the `ios` platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Variant {
    Device,
    Simulator,
    MacCatalyst,
}

impl Variant {
    fn plist_value(&self) -> Option<&'static str> {
        match self {
            Variant::Device => None,
            Variant::Simulator => Some("simulator"),
            Variant::MacCatalyst => Some("maccatalyst"),
        }
    }

    pub fn suffix(&self) -> String {
        self.plist_value()
            .map(|v| format!("-{v}"))
            .unwrap_or_default()
    }
}

/// Architectures are ordered by their Mach-O CPU type, which is the order `lipo` writes fat
/// slices in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Arch {
    X86_64,
    Arm64,
    Arm64_32,
}

impl Arch {
    /// `(cputype, cpusubtype)` from `<mach/machine.h>`.
    pub fn mach_cpu(&self) -> (u32, u32) {
        const CPU_ARCH_ABI64: u32 = 0x0100_0000;
        const CPU_ARCH_ABI64_32: u32 = 0x0200_0000;
        const CPU_TYPE_X86: u32 = 7;
        const CPU_TYPE_ARM: u32 = 12;
        match self {
            Arch::X86_64 => (CPU_TYPE_X86 | CPU_ARCH_ABI64, 3), // CPU_SUBTYPE_X86_ALL
            Arch::Arm64 => (CPU_TYPE_ARM | CPU_ARCH_ABI64, 0),  // CPU_SUBTYPE_ARM64_ALL
            Arch::Arm64_32 => (CPU_TYPE_ARM | CPU_ARCH_ABI64_32, 1), // CPU_SUBTYPE_ARM64_32_V8
        }
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "arm64",
            Arch::Arm64_32 => "arm64_32",
        })
    }
}

/// Works out where a Rust target's library goes, from the target name (e.g. `aarch64-apple-ios-sim`)
/// and its LLVM triple (e.g. `arm64-apple-ios16.0.0-simulator`).
pub fn classify(target: &str, llvm_target: &str) -> Result<(Platform, Variant, Arch)> {
    let mut parts = target.split('-');
    let arch = match parts.next() {
        Some("aarch64") => Arch::Arm64,
        Some("x86_64") => Arch::X86_64,
        Some("arm64_32") => Arch::Arm64_32,
        other => anyhow::bail!("Unsupported architecture {other:?} in target {target}"),
    };
    if parts.next() != Some("apple") {
        anyhow::bail!("{target} is not an Apple target")
    }
    let platform = match parts.next().context("No OS in target")? {
        "darwin" => Platform::MacOS,
        "ios" => Platform::IOS,
        "tvos" => Platform::TvOS,
        "watchos" => Platform::WatchOS,
        os => anyhow::bail!("Unknown Apple OS {os} in target {target}"),
    };
    // `-macabi` targets are named `<arch>-apple-ios-macabi`, but they're a separate library from
    // iOS. Grouping them with iOS would `lipo` two arm64 libraries together.
    let variant = if llvm_target.ends_with("-macabi") {
        Variant::MacCatalyst
    } else if llvm_target.ends_with("-simulator") {
        Variant::Simulator
    } else {
        Variant::Device
    };
    Ok((platform, variant, arch))
}

/// One entry in `AvailableLibraries`.
pub struct Library {
    pub platform: Platform,
    pub variant: Variant,
    /// Sorted, unique.
    pub archs: Vec<Arch>,
    pub binary: String,
    pub headers: String,
}

impl Library {
    /// e.g. `ios-arm64_x86_64-simulator`. `xcodebuild` lists architectures alphabetically here.
    pub fn identifier(&self) -> String {
        let mut archs: Vec<String> = self.archs.iter().map(Arch::to_string).collect();
        archs.sort();
        format!(
            "{}-{}{}",
            self.platform,
            archs.join("_"),
            self.variant.suffix()
        )
    }

    fn plist_dict(&self) -> String {
        let mut archs: Vec<String> = self.archs.iter().map(Arch::to_string).collect();
        archs.sort();
        let archs: String = archs
            .iter()
            .map(|a| format!("\t\t\t\t<string>{a}</string>\n"))
            .collect();
        let variant = self
            .variant
            .plist_value()
            .map(|v| {
                format!("\t\t\t<key>SupportedPlatformVariant</key>\n\t\t\t<string>{v}</string>\n")
            })
            .unwrap_or_default();
        format!(
            "\t\t<dict>\n\
             \t\t\t<key>BinaryPath</key>\n\
             \t\t\t<string>{binary}</string>\n\
             \t\t\t<key>HeadersPath</key>\n\
             \t\t\t<string>{headers}</string>\n\
             \t\t\t<key>LibraryIdentifier</key>\n\
             \t\t\t<string>{identifier}</string>\n\
             \t\t\t<key>LibraryPath</key>\n\
             \t\t\t<string>{binary}</string>\n\
             \t\t\t<key>SupportedArchitectures</key>\n\
             \t\t\t<array>\n\
             {archs}\
             \t\t\t</array>\n\
             \t\t\t<key>SupportedPlatform</key>\n\
             \t\t\t<string>{platform}</string>\n\
             {variant}\
             \t\t</dict>\n",
            binary = self.binary,
            headers = self.headers,
            identifier = self.identifier(),
            platform = self.platform,
        )
    }
}

pub fn info_plist(libraries: &[Library]) -> String {
    let mut libraries: Vec<&Library> = libraries.iter().collect();
    libraries.sort_by_key(|l| l.identifier());
    let dicts: String = libraries.iter().map(|l| l.plist_dict()).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>AvailableLibraries</key>\n\
         \t<array>\n\
         {dicts}\
         \t</array>\n\
         \t<key>CFBundlePackageType</key>\n\
         \t<string>XFWK</string>\n\
         \t<key>XCFrameworkFormatVersion</key>\n\
         \t<string>1.0</string>\n\
         </dict>\n\
         </plist>\n"
    )
}

pub fn write_info_plist(xcframework: &Path, libraries: &[Library]) -> Result<()> {
    let path = xcframework.join("Info.plist");
    std::fs::write(&path, info_plist(libraries))
        .with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(platform: Platform, variant: Variant, archs: Vec<Arch>) -> Library {
        Library {
            platform,
            variant,
            archs,
            binary: "libwp_mobile.a".to_string(),
            headers: "Headers".to_string(),
        }
    }

    #[test]
    fn test_classify() {
        let cases = [
            (
                "aarch64-apple-darwin",
                "arm64-apple-macosx",
                (Platform::MacOS, Variant::Device, Arch::Arm64),
            ),
            (
                "x86_64-apple-ios",
                "x86_64-apple-ios-simulator",
                (Platform::IOS, Variant::Simulator, Arch::X86_64),
            ),
            (
                "aarch64-apple-ios-macabi",
                "arm64-apple-ios-macabi",
                (Platform::IOS, Variant::MacCatalyst, Arch::Arm64),
            ),
            (
                "arm64_32-apple-watchos",
                "arm64_32-apple-watchos",
                (Platform::WatchOS, Variant::Device, Arch::Arm64_32),
            ),
        ];
        for (target, llvm, expected) in cases {
            assert_eq!(classify(target, llvm).unwrap(), expected, "{target}");
        }
        assert!(classify("aarch64-linux-android", "aarch64-linux-android").is_err());
    }

    #[test]
    fn test_identifier_matches_xcodebuild() {
        assert_eq!(
            library(
                Platform::IOS,
                Variant::Simulator,
                vec![Arch::X86_64, Arch::Arm64]
            )
            .identifier(),
            "ios-arm64_x86_64-simulator"
        );
        assert_eq!(
            library(
                Platform::IOS,
                Variant::MacCatalyst,
                vec![Arch::X86_64, Arch::Arm64]
            )
            .identifier(),
            "ios-arm64_x86_64-maccatalyst"
        );
        assert_eq!(
            library(Platform::WatchOS, Variant::Device, vec![Arch::Arm64_32]).identifier(),
            "watchos-arm64_32"
        );
    }

    #[test]
    fn test_info_plist_matches_xcodebuild() {
        let plist = info_plist(&[
            library(Platform::TvOS, Variant::Device, vec![Arch::Arm64]),
            library(
                Platform::IOS,
                Variant::Simulator,
                vec![Arch::X86_64, Arch::Arm64],
            ),
        ]);
        // Copied from `xcodebuild -create-xcframework` output (Xcode 27), with libraries sorted.
        let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AvailableLibraries</key>
	<array>
		<dict>
			<key>BinaryPath</key>
			<string>libwp_mobile.a</string>
			<key>HeadersPath</key>
			<string>Headers</string>
			<key>LibraryIdentifier</key>
			<string>ios-arm64_x86_64-simulator</string>
			<key>LibraryPath</key>
			<string>libwp_mobile.a</string>
			<key>SupportedArchitectures</key>
			<array>
				<string>arm64</string>
				<string>x86_64</string>
			</array>
			<key>SupportedPlatform</key>
			<string>ios</string>
			<key>SupportedPlatformVariant</key>
			<string>simulator</string>
		</dict>
		<dict>
			<key>BinaryPath</key>
			<string>libwp_mobile.a</string>
			<key>HeadersPath</key>
			<string>Headers</string>
			<key>LibraryIdentifier</key>
			<string>tvos-arm64</string>
			<key>LibraryPath</key>
			<string>libwp_mobile.a</string>
			<key>SupportedArchitectures</key>
			<array>
				<string>arm64</string>
			</array>
			<key>SupportedPlatform</key>
			<string>tvos</string>
		</dict>
	</array>
	<key>CFBundlePackageType</key>
	<string>XFWK</string>
	<key>XCFrameworkFormatVersion</key>
	<string>1.0</string>
</dict>
</plist>
"#;
        assert_eq!(plist, expected);
    }
}
