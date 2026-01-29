import Foundation
import WordPressAPIInternal

public extension WpComLanguage {

    init?(locale: Locale) {
        let languageCode = locale.language.languageCode?.identifier
        let regionCode = locale.language.region?.identifier
        let scriptCode = locale.language.script?.identifier

        // Try combined language-region slug first (e.g., "pt-br", "en-gb", "fr-ca")
        if let lang = languageCode, let region = regionCode {
            let combinedSlug = "\(lang)-\(region)".lowercased()
            if let value = wpComLanguageFromSlug(slug: combinedSlug) {
                self = value
                return
            }
        }

        // Handle Chinese script variants (Hans -> zh-cn, Hant -> zh-tw, with regional overrides)
        if languageCode == "zh" {
            if let region = regionCode {
                // Try region-specific Chinese variant first (zh-hk, zh-sg)
                let regionSlug = "zh-\(region)".lowercased()
                if let value = wpComLanguageFromSlug(slug: regionSlug) {
                    self = value
                    return
                }
            }
            // Fall back to script-based variant
            if scriptCode == "Hans" {
                self = WPComLanguage.chineseSimplified
                return
            } else if scriptCode == "Hant" {
                self = WPComLanguage.chineseTraditional
                return
            }
        }

        // Try language code alone via the languageCode initializer
        if let code = locale.language.languageCode, let value = WpComLanguage(languageCode: code) {
            self = value
            return
        }

        // Try the full locale identifier with underscore replaced by hyphen
        let normalizedIdentifier = locale.identifier.replacingOccurrences(of: "_", with: "-").lowercased()
        if let value = wpComLanguageFromSlug(slug: normalizedIdentifier) {
            self = value
            return
        }

        // Final fallback: try the raw locale identifier
        guard let selfValue = wpComLanguageFromSlug(slug: locale.identifier) else {
            return nil
        }

        self = selfValue
    }

    init?(languageCode: Locale.LanguageCode) {
        // Manual overrides to handle differences between WP.com and iOS – WP.com used ISO 639-2 and iOS uses 639-1:
        switch languageCode {
        case .belarusian: // ISO 639-1: be, ISO 639-2: bel
            self = WPComLanguage.belarusian
            return
        case .chinese:
            self = WPComLanguage.chineseSimplified
            return
        case .kurdish: // This is the closest match we have for the moment
            self = WPComLanguage.centralKurdish
            return
        case .sindhi: // ISO 639-1: sd, ISO 639-2: snd
            self = WPComLanguage.sindhi
            return
        case .kyrgyz: // ISO 639-1: ky, ISO 639-2: kir
            self = WPComLanguage.kyrgyz
            return

        default: break
        }

        guard let selfValue = wpComLanguageFromSlug(slug: languageCode.identifier) else {
            return nil
        }

        self = selfValue
    }

    init(id: UInt16) {
        guard let selfValue = wpComLanguageFromId(id: id) else {
            preconditionFailure("Invalid WP.com Language ID: \(id)")
        }
        self = selfValue
    }

    init(slug: String) {
        guard let selfValue = wpComLanguageFromSlug(slug: slug) else {
            preconditionFailure("Invalid WP.com Language slug: \(slug)")
        }
        self = selfValue
    }

    static var all: [WpComLanguage] {
        wpComLanguageAll()
    }

    static var popular: [WpComLanguage] {
        wpComLanguagePopular()
    }
}
