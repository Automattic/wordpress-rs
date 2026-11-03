#if canImport(CoreGraphics)
import CoreGraphics
import Testing
import WordPressAPI

#if canImport(UIKit)
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif

#if canImport(SwiftUI)
import SwiftUI
#endif

struct ReaderColorExtensionsTests {

    @Test
    func readsAndWritesSrgbColors() throws {
        let color = ReaderColor(red: 0x38, green: 0x58, blue: 0xe9, alpha: 0x80)
        #expect(ReaderColor(color.cgColor) == color)
        #expect(color.hex() == "#3858e980")
    }

    @Test
    func convertsOtherColorSpacesToSrgb() throws {
        let gray = try #require(ReaderColor(CGColor(gray: 1, alpha: 1)))
        #expect(gray == ReaderColor(red: 255, green: 255, blue: 255, alpha: 255))

        // Display P3's components don't mean the same as sRGB's, so reading them as sRGB would
        // be wrong.
        let displayP3 = try #require(CGColorSpace(name: CGColorSpace.displayP3))
        let p3Color = try #require(CGColor(colorSpace: displayP3, components: [0.2, 0.6, 0.3, 1]))
        let color = try #require(ReaderColor(p3Color))
        #expect(color != ReaderColor(red: 51, green: 153, blue: 77, alpha: 255))
        #expect(color.green > color.red && color.green > color.blue)
    }

    #if canImport(UIKit) && !os(watchOS)
    @Test
    func readsUIKitColorsForEachAppearance() throws {
        let light = try #require(ReaderColor(.label, traits: UITraitCollection(userInterfaceStyle: .light)))
        let dark = try #require(ReaderColor(.label, traits: UITraitCollection(userInterfaceStyle: .dark)))
        #expect(light.isOpaqueOrTranslucent(red: 0, green: 0, blue: 0))
        #expect(dark.isOpaqueOrTranslucent(red: 255, green: 255, blue: 255))
        #expect(ReaderColor(light.uiColor, traits: UITraitCollection()) == light)
    }
    #elseif canImport(AppKit)
    @Test
    func readsAppKitColorsForEachAppearance() throws {
        let lightAppearance = try #require(NSAppearance(named: .aqua))
        let darkAppearance = try #require(NSAppearance(named: .darkAqua))
        let light = try #require(ReaderColor(.textColor, appearance: lightAppearance))
        let dark = try #require(ReaderColor(.textColor, appearance: darkAppearance))
        #expect(light == ReaderColor(red: 0, green: 0, blue: 0, alpha: 255))
        #expect(dark == ReaderColor(red: 255, green: 255, blue: 255, alpha: 255))
        #expect(ReaderColor(light.nsColor, appearance: darkAppearance) == light)
    }
    #endif

    #if canImport(SwiftUI)
    @Test
    @available(iOS 17, macOS 14, tvOS 17, watchOS 10, *)
    @MainActor
    func readsSwiftUIColorsForEachAppearance() throws {
        var lightEnvironment = EnvironmentValues()
        lightEnvironment.colorScheme = .light
        var darkEnvironment = EnvironmentValues()
        darkEnvironment.colorScheme = .dark

        let color = ReaderColor(red: 0x38, green: 0x58, blue: 0xe9, alpha: 0x80)
        #expect(ReaderColor(color.color, in: lightEnvironment) == color)

        #if canImport(UIKit) && !os(watchOS)
        let system = Color(uiColor: .label)
        #elseif canImport(AppKit)
        let system = Color(nsColor: .textColor)
        #else
        let system = Color.primary
        #endif
        let light = try #require(ReaderColor(system, in: lightEnvironment))
        let dark = try #require(ReaderColor(system, in: darkEnvironment))
        #expect(light.isOpaqueOrTranslucent(red: 0, green: 0, blue: 0))
        #expect(dark.isOpaqueOrTranslucent(red: 255, green: 255, blue: 255))
    }
    #endif

}

private extension ReaderColor {
    /// Whether the colour is this one at any opacity. A system text colour isn't always opaque: on
    /// CI's iOS 26 simulators, `.label` read with only a light or dark style was 85% opaque in one
    /// run, and opaque in the one before.
    func isOpaqueOrTranslucent(red: UInt8, green: UInt8, blue: UInt8) -> Bool {
        self.red == red && self.green == green && self.blue == blue && alpha > 0
    }
}

#endif
