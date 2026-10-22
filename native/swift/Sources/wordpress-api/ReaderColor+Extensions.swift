import CoreGraphics
import WordPressAPIInternal

#if canImport(UIKit)
import UIKit
#endif

public extension ReaderColor {

    /// The colour in sRGB, from a Core Graphics colour in any colour space.
    ///
    /// A colour in a wider colour space, like Display P3, is converted to sRGB first, and what sRGB
    /// can't show is clamped. A colour that can't be converted, like a pattern, gives `nil`.
    init?(_ color: CGColor) {
        guard
            let sRGB = CGColorSpace(name: CGColorSpace.sRGB),
            let converted = color.converted(to: sRGB, intent: .defaultIntent, options: nil),
            let components = converted.components,
            components.count == 4
        else {
            return nil
        }
        self = readerColorFromComponents(
            red: Double(components[0]),
            green: Double(components[1]),
            blue: Double(components[2]),
            alpha: Double(components[3])
        )
    }

    /// The colour as a Core Graphics colour, in sRGB.
    var cgColor: CGColor {
        CGColor(
            srgbRed: CGFloat(red) / 255,
            green: CGFloat(green) / 255,
            blue: CGFloat(blue) / 255,
            alpha: CGFloat(alpha) / 255
        )
    }

}

#if canImport(UIKit) && !os(watchOS)
public extension ReaderColor {

    /// The colour in sRGB, from a UIKit colour as it looks with `traits`.
    ///
    /// System colours like `.label` look different in light and dark mode, so read each one for
    /// the appearance you want, like `UITraitCollection(userInterfaceStyle: .dark)`.
    init?(_ color: UIColor, traits: UITraitCollection) {
        self.init(color.resolvedColor(with: traits).cgColor)
    }

    /// The colour as a UIKit colour.
    var uiColor: UIColor {
        UIColor(cgColor: cgColor)
    }

}
#endif
