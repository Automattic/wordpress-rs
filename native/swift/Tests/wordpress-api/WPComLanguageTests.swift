import Foundation
import Testing
import WordPressAPI

@Suite
struct WPComLanguageTests {

    // All Locale.LanguageCode static properties that match WPComLanguage cases
    @Test("Named Language Code Check", arguments: [
        // A
        (Locale.LanguageCode.arabic, WPComLanguage.arabic),
        (Locale.LanguageCode.armenian, WPComLanguage.armenian),
        (Locale.LanguageCode.assamese, WPComLanguage.assamese),
        (Locale.LanguageCode.azerbaijani, WPComLanguage.azerbaijani),
        // B
        (Locale.LanguageCode.bangla, WPComLanguage.bengali),
        (Locale.LanguageCode.bulgarian, WPComLanguage.bulgarian),
        (Locale.LanguageCode.belarusian, WPComLanguage.belarusian),
        // C
        (Locale.LanguageCode.catalan, WPComLanguage.catalan),
        (Locale.LanguageCode.chinese, WPComLanguage.chineseSimplified),
        (Locale.LanguageCode.croatian, WPComLanguage.croatian),
        (Locale.LanguageCode.czech, WPComLanguage.czech),
        // D
        (Locale.LanguageCode.danish, WPComLanguage.danish),
        (Locale.LanguageCode.dutch, WPComLanguage.dutch),
        (Locale.LanguageCode.dzongkha, WPComLanguage.dzongkha),
        // E
        (Locale.LanguageCode.english, WPComLanguage.english),
        (Locale.LanguageCode.estonian, WPComLanguage.estonian),
        // F
        (Locale.LanguageCode.faroese, WPComLanguage.faroese),
        (Locale.LanguageCode.finnish, WPComLanguage.finnish),
        (Locale.LanguageCode.french, WPComLanguage.french),
        // G
        (Locale.LanguageCode.georgian, WPComLanguage.georgian),
        (Locale.LanguageCode.german, WPComLanguage.german),
        (Locale.LanguageCode.greek, WPComLanguage.greek),
        (Locale.LanguageCode.gujarati, WPComLanguage.gujarati),
        // H
        (Locale.LanguageCode.hebrew, WPComLanguage.hebrew),
        (Locale.LanguageCode.hindi, WPComLanguage.hindi),
        (Locale.LanguageCode.hungarian, WPComLanguage.hungarian),
        // I
        (Locale.LanguageCode.icelandic, WPComLanguage.icelandic),
        (Locale.LanguageCode.indonesian, WPComLanguage.indonesian),
        (Locale.LanguageCode.irish, WPComLanguage.irish),
        (Locale.LanguageCode.italian, WPComLanguage.italian),
        // J
        (Locale.LanguageCode.japanese, WPComLanguage.japanese),
        // K
        (Locale.LanguageCode.kannada, WPComLanguage.kannada),
        (Locale.LanguageCode.kazakh, WPComLanguage.kazakh),
        (Locale.LanguageCode.khmer, WPComLanguage.khmer),
        (Locale.LanguageCode.korean, WPComLanguage.korean),
        (Locale.LanguageCode.kurdish, WPComLanguage.centralKurdish),
        (Locale.LanguageCode.kyrgyz, WPComLanguage.kyrgyz),
        // L
        (Locale.LanguageCode.lao, WPComLanguage.lao),
        (Locale.LanguageCode.latvian, WPComLanguage.latvian),
        (Locale.LanguageCode.lithuanian, WPComLanguage.lithuanian),
        // M
        (Locale.LanguageCode.macedonian, WPComLanguage.macedonian),
        (Locale.LanguageCode.malay, WPComLanguage.malay),
        (Locale.LanguageCode.malayalam, WPComLanguage.malayalam),
        (Locale.LanguageCode.maltese, WPComLanguage.maltese),
        (Locale.LanguageCode.marathi, WPComLanguage.marathi),
        (Locale.LanguageCode.mongolian, WPComLanguage.mongolian),
        // N
        (Locale.LanguageCode.nepali, WPComLanguage.nepali),
        (Locale.LanguageCode.norwegianBokmål, WPComLanguage.norwegianBokmal),
        (Locale.LanguageCode.norwegianNynorsk, WPComLanguage.norwegianNynorsk),
        // O
        (Locale.LanguageCode.odia, WPComLanguage.odia),
        // P
        (Locale.LanguageCode.pashto, WPComLanguage.pashto),
        (Locale.LanguageCode.persian, WPComLanguage.persian),
        (Locale.LanguageCode.polish, WPComLanguage.polish),
        (Locale.LanguageCode.portuguese, WPComLanguage.portuguese),
        (Locale.LanguageCode.punjabi, WPComLanguage.punjabi),
        // R
        (Locale.LanguageCode.romanian, WPComLanguage.romanian),
        (Locale.LanguageCode.russian, WPComLanguage.russian),
        // S
        (Locale.LanguageCode.serbian, WPComLanguage.serbian),
        (Locale.LanguageCode.sindhi, WPComLanguage.sindhi),
        (Locale.LanguageCode.sinhala, WPComLanguage.sinhala),
        (Locale.LanguageCode.slovak, WPComLanguage.slovak),
        (Locale.LanguageCode.slovenian, WPComLanguage.slovenian),
        (Locale.LanguageCode.spanish, WPComLanguage.spanish),
        (Locale.LanguageCode.swedish, WPComLanguage.swedish),
        // T
        (Locale.LanguageCode.tagalog, WPComLanguage.tagalog),
        (Locale.LanguageCode.tamil, WPComLanguage.tamil),
        (Locale.LanguageCode.telugu, WPComLanguage.telugu),
        (Locale.LanguageCode.thai, WPComLanguage.thai),
        (Locale.LanguageCode.tibetan, WPComLanguage.tibetan),
        (Locale.LanguageCode.turkish, WPComLanguage.turkish),
        // U
        (Locale.LanguageCode.ukrainian, WPComLanguage.ukrainian),
        (Locale.LanguageCode.urdu, WPComLanguage.urdu),
        (Locale.LanguageCode.uyghur, WPComLanguage.uyghur),
        (Locale.LanguageCode.uzbek, WPComLanguage.uzbek),
        // V
        (Locale.LanguageCode.vietnamese, WPComLanguage.vietnamese),
        // W
        (Locale.LanguageCode.welsh, WPComLanguage.welsh),
        // Y
        (Locale.LanguageCode.yiddish, WPComLanguage.yiddish)
    ])
    func namedLanguageCodeCheck(code: Locale.LanguageCode, expected: WPComLanguage) async throws {
        #expect(WPComLanguage(languageCode: code) == expected)
    }

    // Locale identifiers from Locale.availableIdentifiers that match WPComLanguage cases
    @Test("Common Locale Check", arguments: [
        // A
        (Locale(identifier: "af"), WPComLanguage.afrikaans),
        (Locale(identifier: "am"), WPComLanguage.amharic),
        (Locale(identifier: "ar"), WPComLanguage.arabic),
        (Locale(identifier: "as"), WPComLanguage.assamese),
        (Locale(identifier: "ast"), WPComLanguage.asturian),
        (Locale(identifier: "az"), WPComLanguage.azerbaijani),
        // B
        (Locale(identifier: "ba"), WPComLanguage.bashkir),
        (Locale(identifier: "bg"), WPComLanguage.bulgarian),
        (Locale(identifier: "bm"), WPComLanguage.bambara),
        (Locale(identifier: "bn"), WPComLanguage.bengali),
        (Locale(identifier: "bo"), WPComLanguage.tibetan),
        (Locale(identifier: "br"), WPComLanguage.breton),
        (Locale(identifier: "bs"), WPComLanguage.bosnian),
        // C
        (Locale(identifier: "ca"), WPComLanguage.catalan),
        (Locale(identifier: "ce"), WPComLanguage.chechen),
        (Locale(identifier: "ckb"), WPComLanguage.centralKurdish),
        (Locale(identifier: "cs"), WPComLanguage.czech),
        (Locale(identifier: "cv"), WPComLanguage.chuvash),
        (Locale(identifier: "cy"), WPComLanguage.welsh),
        // D
        (Locale(identifier: "da"), WPComLanguage.danish),
        (Locale(identifier: "de"), WPComLanguage.german),
        (Locale(identifier: "dv"), WPComLanguage.dhivehi),
        (Locale(identifier: "dz"), WPComLanguage.dzongkha),
        // E
        (Locale(identifier: "el"), WPComLanguage.greek),
        (Locale(identifier: "en"), WPComLanguage.english),
        (Locale(identifier: "eo"), WPComLanguage.esperanto),
        (Locale(identifier: "es"), WPComLanguage.spanish),
        (Locale(identifier: "et"), WPComLanguage.estonian),
        (Locale(identifier: "eu"), WPComLanguage.basque),
        // F
        (Locale(identifier: "fa"), WPComLanguage.persian),
        (Locale(identifier: "fi"), WPComLanguage.finnish),
        (Locale(identifier: "fo"), WPComLanguage.faroese),
        (Locale(identifier: "fr"), WPComLanguage.french),
        (Locale(identifier: "fur"), WPComLanguage.friulian),
        (Locale(identifier: "fy"), WPComLanguage.westernFrisian),
        // G
        (Locale(identifier: "ga"), WPComLanguage.irish),
        (Locale(identifier: "gd"), WPComLanguage.scottishGaelic),
        (Locale(identifier: "gl"), WPComLanguage.galician),
        (Locale(identifier: "gn"), WPComLanguage.guarani),
        (Locale(identifier: "gu"), WPComLanguage.gujarati),
        // H
        (Locale(identifier: "he"), WPComLanguage.hebrew),
        (Locale(identifier: "hi"), WPComLanguage.hindi),
        (Locale(identifier: "hr"), WPComLanguage.croatian),
        (Locale(identifier: "hu"), WPComLanguage.hungarian),
        (Locale(identifier: "hy"), WPComLanguage.armenian),
        // I
        (Locale(identifier: "ia"), WPComLanguage.interlingua),
        (Locale(identifier: "id"), WPComLanguage.indonesian),
        (Locale(identifier: "ii"), WPComLanguage.nuosuYi),
        (Locale(identifier: "is"), WPComLanguage.icelandic),
        (Locale(identifier: "it"), WPComLanguage.italian),
        // J
        (Locale(identifier: "ja"), WPComLanguage.japanese),
        // K
        (Locale(identifier: "ka"), WPComLanguage.georgian),
        (Locale(identifier: "kab"), WPComLanguage.kabyle),
        (Locale(identifier: "kk"), WPComLanguage.kazakh),
        (Locale(identifier: "km"), WPComLanguage.khmer),
        (Locale(identifier: "kn"), WPComLanguage.kannada),
        (Locale(identifier: "ko"), WPComLanguage.korean),
        (Locale(identifier: "ks"), WPComLanguage.kashmiri),
        // L
        (Locale(identifier: "lo"), WPComLanguage.lao),
        (Locale(identifier: "lt"), WPComLanguage.lithuanian),
        (Locale(identifier: "lv"), WPComLanguage.latvian),
        // M
        (Locale(identifier: "mk"), WPComLanguage.macedonian),
        (Locale(identifier: "ml"), WPComLanguage.malayalam),
        (Locale(identifier: "mn"), WPComLanguage.mongolian),
        (Locale(identifier: "mr"), WPComLanguage.marathi),
        (Locale(identifier: "ms"), WPComLanguage.malay),
        (Locale(identifier: "mt"), WPComLanguage.maltese),
        // N
        (Locale(identifier: "nb"), WPComLanguage.norwegianBokmal),
        (Locale(identifier: "nds"), WPComLanguage.lowGerman),
        (Locale(identifier: "ne"), WPComLanguage.nepali),
        (Locale(identifier: "nl"), WPComLanguage.dutch),
        (Locale(identifier: "nn"), WPComLanguage.norwegianNynorsk),
        (Locale(identifier: "nv"), WPComLanguage.navajo),
        // O
        (Locale(identifier: "or"), WPComLanguage.odia),
        (Locale(identifier: "os"), WPComLanguage.ossetic),
        // P
        (Locale(identifier: "pa"), WPComLanguage.punjabi),
        (Locale(identifier: "pl"), WPComLanguage.polish),
        (Locale(identifier: "ps"), WPComLanguage.pashto),
        (Locale(identifier: "pt"), WPComLanguage.portuguese),
        // Q
        (Locale(identifier: "qu"), WPComLanguage.quechua),
        // R
        (Locale(identifier: "ro"), WPComLanguage.romanian),
        (Locale(identifier: "ru"), WPComLanguage.russian),
        // S
        (Locale(identifier: "sc"), WPComLanguage.sardinian),
        (Locale(identifier: "si"), WPComLanguage.sinhala),
        (Locale(identifier: "sk"), WPComLanguage.slovak),
        (Locale(identifier: "sl"), WPComLanguage.slovenian),
        (Locale(identifier: "so"), WPComLanguage.somali),
        (Locale(identifier: "sq"), WPComLanguage.albanian),
        (Locale(identifier: "sr"), WPComLanguage.serbian),
        (Locale(identifier: "su"), WPComLanguage.sundanese),
        (Locale(identifier: "sv"), WPComLanguage.swedish),
        // T
        (Locale(identifier: "ta"), WPComLanguage.tamil),
        (Locale(identifier: "te"), WPComLanguage.telugu),
        (Locale(identifier: "th"), WPComLanguage.thai),
        (Locale(identifier: "tr"), WPComLanguage.turkish),
        (Locale(identifier: "tt"), WPComLanguage.tatar),
        // U
        (Locale(identifier: "ug"), WPComLanguage.uyghur),
        (Locale(identifier: "uk"), WPComLanguage.ukrainian),
        (Locale(identifier: "ur"), WPComLanguage.urdu),
        (Locale(identifier: "uz"), WPComLanguage.uzbek),
        // V
        (Locale(identifier: "vec"), WPComLanguage.venetian),
        (Locale(identifier: "vi"), WPComLanguage.vietnamese),
        // W
        (Locale(identifier: "wa"), WPComLanguage.walloon),
        // Y
        (Locale(identifier: "yi"), WPComLanguage.yiddish),
        (Locale(identifier: "yo"), WPComLanguage.yoruba),
        // Z
        (Locale(identifier: "za"), WPComLanguage.zhuang)
    ])
    func namedLocaleCheck(locale: Locale, expected: WPComLanguage) async throws {
        #expect(WPComLanguage(locale: locale) == expected)
    }

    // Regional locale variants that map to specific WPComLanguage cases
    @Test("Regional Locale Check", arguments: [
        // Chinese variants
        (Locale(identifier: "zh_Hans"), WPComLanguage.chineseSimplified),
        (Locale(identifier: "zh_Hant"), WPComLanguage.chineseTraditional),
        (Locale(identifier: "zh_Hans_CN"), WPComLanguage.chineseSimplified),
        (Locale(identifier: "zh_Hant_TW"), WPComLanguage.chineseTraditional),
        (Locale(identifier: "zh_Hant_HK"), WPComLanguage.chineseHongKong),
        (Locale(identifier: "zh_Hans_SG"), WPComLanguage.chineseSingapore),
        // English variants
        (Locale(identifier: "en_GB"), WPComLanguage.englishUk),
        (Locale(identifier: "en_US"), WPComLanguage.english),
        (Locale(identifier: "en_AU"), WPComLanguage.english),
        // French variants
        (Locale(identifier: "fr_CA"), WPComLanguage.frenchCanada),
        (Locale(identifier: "fr_CH"), WPComLanguage.frenchSwitzerland),
        (Locale(identifier: "fr_BE"), WPComLanguage.frenchBelgium),
        (Locale(identifier: "fr_FR"), WPComLanguage.french),
        // German variants
        (Locale(identifier: "de_AT"), WPComLanguage.germanAustria),
        (Locale(identifier: "de_CH"), WPComLanguage.germanSwitzerland),
        (Locale(identifier: "de_DE"), WPComLanguage.german),
        // Spanish variants
        (Locale(identifier: "es_MX"), WPComLanguage.spanishMexico),
        (Locale(identifier: "es_CL"), WPComLanguage.spanishChile),
        (Locale(identifier: "es_ES"), WPComLanguage.spanish),
        // Portuguese variants
        (Locale(identifier: "pt_BR"), WPComLanguage.portugueseBrazil),
        (Locale(identifier: "pt_PT"), WPComLanguage.portuguese),
        // Dutch variants
        (Locale(identifier: "nl_BE"), WPComLanguage.dutchBelgium),
        (Locale(identifier: "nl_NL"), WPComLanguage.dutch)
    ])
    func regionalLocaleCheck(locale: Locale, expected: WPComLanguage) async throws {
        #expect(WPComLanguage(locale: locale) == expected)
    }

}
