import CoreGraphics
import Testing
import WordPressAPI

#if canImport(UIKit)
import UIKit
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
        #expect(light == ReaderColor(red: 0, green: 0, blue: 0, alpha: 255))
        #expect(dark == ReaderColor(red: 255, green: 255, blue: 255, alpha: 255))
        #expect(ReaderColor(light.uiColor, traits: UITraitCollection()) == light)
    }
    #endif

}
