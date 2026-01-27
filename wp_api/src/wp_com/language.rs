use std::collections::HashMap;

use crate::url_query::{AppendUrlQueryPairs, QueryPairs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpComRemoteLanguage {
    pub id: u16,
    pub localized: String,
    pub name: String,
    pub popularity_rank: Option<u16>,
}

pub type WpComRemoteLanguageMap = HashMap<String, WpComRemoteLanguage>;

/// Parameters for fetching language names from the WordPress.com API.
#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct LanguagesGetParams {
    /// The locale to use for returning language names.
    /// When specified, language names will be returned in this locale.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
}

impl AppendUrlQueryPairs for LanguagesGetParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        if let Some(locale) = &self.locale {
            query_pairs_mut.append_pair("_locale", locale.slug_str());
        }
    }
}

/// WordPress.com language codes with their IDs as discriminants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
#[repr(u16)]
pub enum WPComLanguage {
    English = 1,
    Afrikaans = 2,
    Arabic = 3,
    Assamese = 4,
    Belarusian = 5,
    Bulgarian = 6,
    Bambara = 7,
    Bengali = 8,
    Tibetan = 9,
    Catalan = 10,
    Czech = 11,
    Kashubian = 12,
    Welsh = 13,
    Danish = 14,
    German = 15,
    Dzongkha = 16,
    Greek = 17,
    Esperanto = 18,
    Spanish = 19,
    Estonian = 20,
    Persian = 21,
    Finnish = 22,
    Faroese = 23,
    French = 24,
    Friulian = 25,
    WesternFrisian = 26,
    Irish = 27,
    Gujarati = 28,
    Hebrew = 29,
    Hindi = 30,
    Hungarian = 31,
    Interlingua = 32,
    Indonesian = 33,
    Icelandic = 34,
    Italian = 35,
    Japanese = 36,
    Georgian = 37,
    Khmer = 38,
    Kannada = 39,
    Korean = 40,
    CentralKurdish = 41,
    Latin = 42,
    Limburgish = 43,
    Lao = 44,
    Lithuanian = 45,
    Malayalam = 46,
    Malay = 47,
    LowGerman = 48,
    Dutch = 49,
    NorwegianNynorsk = 50,
    NorwegianBokmal = 51,
    OldNorse = 52,
    Navajo = 53,
    Occitan = 54,
    Odia = 55,
    Ossetic = 56,
    Punjabi = 57,
    Polish = 58,
    Pashto = 59,
    Portuguese = 60,
    Romanian = 61,
    Russian = 62,
    Sardinian = 63,
    Slovak = 64,
    Slovenian = 65,
    Albanian = 66,
    Serbian = 67,
    Swedish = 68,
    Tamil = 69,
    Telugu = 70,
    Thai = 71,
    Tatar = 72,
    Ukrainian = 73,
    Urdu = 74,
    Walloon = 75,
    Yiddish = 76,
    Turkish = 78,
    Azerbaijani = 79,
    Alemannic = 418,
    Aramaic = 419,
    Asturian = 420,
    Avar = 421,
    Aymara = 422,
    Bashkir = 423,
    Breton = 424,
    Chechen = 425,
    Chuvash = 426,
    Dhivehi = 427,
    Basque = 429,
    Guarani = 430,
    Croatian = 431,
    NuosuYi = 432,
    Kashmiri = 433,
    Komi = 434,
    Macedonian = 435,
    Nahuatl = 436,
    Neapolitan = 437,
    PortugueseBrazil = 438,
    Quechua = 439,
    Sindhi = 440,
    Sundanese = 441,
    Tahitian = 442,
    Udmurt = 443,
    Uyghur = 444,
    Venetian = 445,
    Vietnamese = 446,
    Kalmyk = 447,
    Zhuang = 448,
    ChineseSimplified = 449,
    ChineseTraditional = 452,
    Latvian = 453,
    Bosnian = 454,
    Tagalog = 455,
    Nepali = 456,
    Galician = 457,
    Uzbek = 458,
    Somali = 459,
    Marathi = 461,
    Kazakh = 462,
    Mirandese = 464,
    Maltese = 465,
    Armenian = 467,
    GreekPolytonic = 468,
    Iloko = 469,
    Sinhala = 471,
    Mongolian = 472,
    FrenchSwitzerland = 474,
    FrenchCanada = 475,
    ScottishGaelic = 476,
    Yoruba = 477,
    FrenchBelgium = 478,
    Kyrgyz = 479,
    Tigrinya = 480,
    Amharic = 481,
    EnglishUK = 482,
    Aromanian = 483,
    SpanishChile = 484,
    Aragonese = 485,
    Saraiki = 486,
    ChineseHongKong = 487,
    Kabyle = 488,
    ChineseSingapore = 489,
    Kalaallisut = 490,
    GermanFormal = 900,
    SerbianLatin = 901,
    SpanishMexico = 902,
    GermanSwitzerland = 903,
    GermanAustria = 905,
    DutchBelgium = 906,
}

// UniFFI-exported instance methods for WPComLanguage
// Note: Static/constructor methods are exposed as standalone functions below
// since UniFFI doesn't support constructors on enums
#[uniffi::export]
impl WPComLanguage {
    /// Returns the language slug (e.g., "en", "es", "pt-br")
    pub fn slug(&self) -> String {
        self.slug_str().to_string()
    }

    /// Returns the display name in the native language
    pub fn name(&self) -> String {
        self.name_str().to_string()
    }

    /// Returns the WordPress locale code (e.g., "en_US", "es_ES")
    /// Returns an empty string if no WordPress locale is available
    pub fn wp_locale(&self) -> String {
        self.wp_locale_str().to_string()
    }

    /// Returns the UN M.49 territory codes associated with this language
    pub fn territories(&self) -> Vec<String> {
        self.territories_slice()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Returns the popularity rank if this is a popular language
    /// Returns None if the language is not marked as popular
    pub fn popular_rank(&self) -> Option<u8> {
        self.popular_rank_internal()
    }

    /// Returns the parent locale slug if this is a variant
    pub fn parent_locale_slug(&self) -> Option<String> {
        self.parent_locale_slug_str().map(|s| s.to_string())
    }

    /// Returns the numeric language ID as used by WordPress.com
    pub fn language_id(&self) -> u16 {
        *self as u16
    }
}

/// Lookup a WPComLanguage by its numeric ID
#[uniffi::export]
pub fn wp_com_language_from_id(id: u16) -> Option<WPComLanguage> {
    WPComLanguage::from_id(id)
}

/// Lookup a WPComLanguage by its slug (e.g., "en", "pt-br")
#[uniffi::export]
pub fn wp_com_language_from_slug(slug: String) -> Option<WPComLanguage> {
    WPComLanguage::from_slug(&slug)
}

/// Returns all available languages
#[uniffi::export]
pub fn wp_com_language_all() -> Vec<WPComLanguage> {
    WPComLanguage::all()
}

/// Returns only the popular languages, sorted by popularity rank
#[uniffi::export]
pub fn wp_com_language_popular() -> Vec<WPComLanguage> {
    WPComLanguage::popular()
}

impl WPComLanguage {
    /// Lookup a language by its numeric ID (discriminant)
    pub fn from_id(id: u16) -> Option<Self> {
        Self::from_discriminant(id)
    }

    /// Lookup a language by its discriminant value
    pub fn from_discriminant(id: u16) -> Option<Self> {
        match id {
            1 => Some(Self::English),
            2 => Some(Self::Afrikaans),
            3 => Some(Self::Arabic),
            4 => Some(Self::Assamese),
            5 => Some(Self::Belarusian),
            6 => Some(Self::Bulgarian),
            7 => Some(Self::Bambara),
            8 => Some(Self::Bengali),
            9 => Some(Self::Tibetan),
            10 => Some(Self::Catalan),
            11 => Some(Self::Czech),
            12 => Some(Self::Kashubian),
            13 => Some(Self::Welsh),
            14 => Some(Self::Danish),
            15 => Some(Self::German),
            16 => Some(Self::Dzongkha),
            17 => Some(Self::Greek),
            18 => Some(Self::Esperanto),
            19 => Some(Self::Spanish),
            20 => Some(Self::Estonian),
            21 => Some(Self::Persian),
            22 => Some(Self::Finnish),
            23 => Some(Self::Faroese),
            24 => Some(Self::French),
            25 => Some(Self::Friulian),
            26 => Some(Self::WesternFrisian),
            27 => Some(Self::Irish),
            28 => Some(Self::Gujarati),
            29 => Some(Self::Hebrew),
            30 => Some(Self::Hindi),
            31 => Some(Self::Hungarian),
            32 => Some(Self::Interlingua),
            33 => Some(Self::Indonesian),
            34 => Some(Self::Icelandic),
            35 => Some(Self::Italian),
            36 => Some(Self::Japanese),
            37 => Some(Self::Georgian),
            38 => Some(Self::Khmer),
            39 => Some(Self::Kannada),
            40 => Some(Self::Korean),
            41 => Some(Self::CentralKurdish),
            42 => Some(Self::Latin),
            43 => Some(Self::Limburgish),
            44 => Some(Self::Lao),
            45 => Some(Self::Lithuanian),
            46 => Some(Self::Malayalam),
            47 => Some(Self::Malay),
            48 => Some(Self::LowGerman),
            49 => Some(Self::Dutch),
            50 => Some(Self::NorwegianNynorsk),
            51 => Some(Self::NorwegianBokmal),
            52 => Some(Self::OldNorse),
            53 => Some(Self::Navajo),
            54 => Some(Self::Occitan),
            55 => Some(Self::Odia),
            56 => Some(Self::Ossetic),
            57 => Some(Self::Punjabi),
            58 => Some(Self::Polish),
            59 => Some(Self::Pashto),
            60 => Some(Self::Portuguese),
            61 => Some(Self::Romanian),
            62 => Some(Self::Russian),
            63 => Some(Self::Sardinian),
            64 => Some(Self::Slovak),
            65 => Some(Self::Slovenian),
            66 => Some(Self::Albanian),
            67 => Some(Self::Serbian),
            68 => Some(Self::Swedish),
            69 => Some(Self::Tamil),
            70 => Some(Self::Telugu),
            71 => Some(Self::Thai),
            72 => Some(Self::Tatar),
            73 => Some(Self::Ukrainian),
            74 => Some(Self::Urdu),
            75 => Some(Self::Walloon),
            76 => Some(Self::Yiddish),
            78 => Some(Self::Turkish),
            79 => Some(Self::Azerbaijani),
            418 => Some(Self::Alemannic),
            419 => Some(Self::Aramaic),
            420 => Some(Self::Asturian),
            421 => Some(Self::Avar),
            422 => Some(Self::Aymara),
            423 => Some(Self::Bashkir),
            424 => Some(Self::Breton),
            425 => Some(Self::Chechen),
            426 => Some(Self::Chuvash),
            427 => Some(Self::Dhivehi),
            429 => Some(Self::Basque),
            430 => Some(Self::Guarani),
            431 => Some(Self::Croatian),
            432 => Some(Self::NuosuYi),
            433 => Some(Self::Kashmiri),
            434 => Some(Self::Komi),
            435 => Some(Self::Macedonian),
            436 => Some(Self::Nahuatl),
            437 => Some(Self::Neapolitan),
            438 => Some(Self::PortugueseBrazil),
            439 => Some(Self::Quechua),
            440 => Some(Self::Sindhi),
            441 => Some(Self::Sundanese),
            442 => Some(Self::Tahitian),
            443 => Some(Self::Udmurt),
            444 => Some(Self::Uyghur),
            445 => Some(Self::Venetian),
            446 => Some(Self::Vietnamese),
            447 => Some(Self::Kalmyk),
            448 => Some(Self::Zhuang),
            449 => Some(Self::ChineseSimplified),
            452 => Some(Self::ChineseTraditional),
            453 => Some(Self::Latvian),
            454 => Some(Self::Bosnian),
            455 => Some(Self::Tagalog),
            456 => Some(Self::Nepali),
            457 => Some(Self::Galician),
            458 => Some(Self::Uzbek),
            459 => Some(Self::Somali),
            461 => Some(Self::Marathi),
            462 => Some(Self::Kazakh),
            464 => Some(Self::Mirandese),
            465 => Some(Self::Maltese),
            467 => Some(Self::Armenian),
            468 => Some(Self::GreekPolytonic),
            469 => Some(Self::Iloko),
            471 => Some(Self::Sinhala),
            472 => Some(Self::Mongolian),
            474 => Some(Self::FrenchSwitzerland),
            475 => Some(Self::FrenchCanada),
            476 => Some(Self::ScottishGaelic),
            477 => Some(Self::Yoruba),
            478 => Some(Self::FrenchBelgium),
            479 => Some(Self::Kyrgyz),
            480 => Some(Self::Tigrinya),
            481 => Some(Self::Amharic),
            482 => Some(Self::EnglishUK),
            483 => Some(Self::Aromanian),
            484 => Some(Self::SpanishChile),
            485 => Some(Self::Aragonese),
            486 => Some(Self::Saraiki),
            487 => Some(Self::ChineseHongKong),
            488 => Some(Self::Kabyle),
            489 => Some(Self::ChineseSingapore),
            490 => Some(Self::Kalaallisut),
            900 => Some(Self::GermanFormal),
            901 => Some(Self::SerbianLatin),
            902 => Some(Self::SpanishMexico),
            903 => Some(Self::GermanSwitzerland),
            // 904 is deliberately omitted
            905 => Some(Self::GermanAustria),
            906 => Some(Self::DutchBelgium),
            _ => None,
        }
    }

    /// Lookup a language by its slug (e.g., "en", "pt-br")
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "en" => Some(Self::English),
            "af" => Some(Self::Afrikaans),
            "ar" => Some(Self::Arabic),
            "as" => Some(Self::Assamese),
            "bel" => Some(Self::Belarusian),
            "bg" => Some(Self::Bulgarian),
            "bm" => Some(Self::Bambara),
            "bn" => Some(Self::Bengali),
            "bo" => Some(Self::Tibetan),
            "ca" => Some(Self::Catalan),
            "cs" => Some(Self::Czech),
            "csb" => Some(Self::Kashubian),
            "cy" => Some(Self::Welsh),
            "da" => Some(Self::Danish),
            "de" => Some(Self::German),
            "dz" => Some(Self::Dzongkha),
            "el" => Some(Self::Greek),
            "eo" => Some(Self::Esperanto),
            "es" => Some(Self::Spanish),
            "et" => Some(Self::Estonian),
            "fa" => Some(Self::Persian),
            "fi" => Some(Self::Finnish),
            "fo" => Some(Self::Faroese),
            "fr" => Some(Self::French),
            "fur" => Some(Self::Friulian),
            "fy" => Some(Self::WesternFrisian),
            "ga" => Some(Self::Irish),
            "gu" => Some(Self::Gujarati),
            "he" => Some(Self::Hebrew),
            "hi" => Some(Self::Hindi),
            "hu" => Some(Self::Hungarian),
            "ia" => Some(Self::Interlingua),
            "id" => Some(Self::Indonesian),
            "is" => Some(Self::Icelandic),
            "it" => Some(Self::Italian),
            "ja" => Some(Self::Japanese),
            "ka" => Some(Self::Georgian),
            "km" => Some(Self::Khmer),
            "kn" => Some(Self::Kannada),
            "ko" => Some(Self::Korean),
            "ckb" => Some(Self::CentralKurdish),
            "la" => Some(Self::Latin),
            "li" => Some(Self::Limburgish),
            "lo" => Some(Self::Lao),
            "lt" => Some(Self::Lithuanian),
            "ml" => Some(Self::Malayalam),
            "ms" => Some(Self::Malay),
            "nds" => Some(Self::LowGerman),
            "nl" => Some(Self::Dutch),
            "nn" => Some(Self::NorwegianNynorsk),
            "nb" => Some(Self::NorwegianBokmal),
            "non" => Some(Self::OldNorse),
            "nv" => Some(Self::Navajo),
            "oci" => Some(Self::Occitan),
            "or" => Some(Self::Odia),
            "os" => Some(Self::Ossetic),
            "pa" => Some(Self::Punjabi),
            "pl" => Some(Self::Polish),
            "ps" => Some(Self::Pashto),
            "pt" => Some(Self::Portuguese),
            "ro" => Some(Self::Romanian),
            "ru" => Some(Self::Russian),
            "sc" => Some(Self::Sardinian),
            "sk" => Some(Self::Slovak),
            "sl" => Some(Self::Slovenian),
            "sq" => Some(Self::Albanian),
            "sr" => Some(Self::Serbian),
            "sv" => Some(Self::Swedish),
            "ta" => Some(Self::Tamil),
            "te" => Some(Self::Telugu),
            "th" => Some(Self::Thai),
            "tt" => Some(Self::Tatar),
            "uk" => Some(Self::Ukrainian),
            "ur" => Some(Self::Urdu),
            "wa" => Some(Self::Walloon),
            "yi" => Some(Self::Yiddish),
            "tr" => Some(Self::Turkish),
            "az" => Some(Self::Azerbaijani),
            "als" => Some(Self::Alemannic),
            "arc" => Some(Self::Aramaic),
            "ast" => Some(Self::Asturian),
            "av" => Some(Self::Avar),
            "ay" => Some(Self::Aymara),
            "ba" => Some(Self::Bashkir),
            "br" => Some(Self::Breton),
            "ce" => Some(Self::Chechen),
            "cv" => Some(Self::Chuvash),
            "dv" => Some(Self::Dhivehi),
            "eu" => Some(Self::Basque),
            "gn" => Some(Self::Guarani),
            "hr" => Some(Self::Croatian),
            "ii" => Some(Self::NuosuYi),
            "ks" => Some(Self::Kashmiri),
            "kv" => Some(Self::Komi),
            "mk" => Some(Self::Macedonian),
            "nah" => Some(Self::Nahuatl),
            "nap" => Some(Self::Neapolitan),
            "pt-br" => Some(Self::PortugueseBrazil),
            "qu" => Some(Self::Quechua),
            "snd" => Some(Self::Sindhi),
            "su" => Some(Self::Sundanese),
            "ty" => Some(Self::Tahitian),
            "udm" => Some(Self::Udmurt),
            "ug" => Some(Self::Uyghur),
            "vec" => Some(Self::Venetian),
            "vi" => Some(Self::Vietnamese),
            "xal" => Some(Self::Kalmyk),
            "za" => Some(Self::Zhuang),
            "zh-cn" => Some(Self::ChineseSimplified),
            "zh-tw" => Some(Self::ChineseTraditional),
            "lv" => Some(Self::Latvian),
            "bs" => Some(Self::Bosnian),
            "tl" => Some(Self::Tagalog),
            "ne" => Some(Self::Nepali),
            "gl" => Some(Self::Galician),
            "uz" => Some(Self::Uzbek),
            "so" => Some(Self::Somali),
            "mr" => Some(Self::Marathi),
            "kk" => Some(Self::Kazakh),
            "mwl" => Some(Self::Mirandese),
            "mt" => Some(Self::Maltese),
            "hy" => Some(Self::Armenian),
            "el-po" => Some(Self::GreekPolytonic),
            "ilo" => Some(Self::Iloko),
            "si" => Some(Self::Sinhala),
            "mn" => Some(Self::Mongolian),
            "fr-ch" => Some(Self::FrenchSwitzerland),
            "fr-ca" => Some(Self::FrenchCanada),
            "gd" => Some(Self::ScottishGaelic),
            "yo" => Some(Self::Yoruba),
            "fr-be" => Some(Self::FrenchBelgium),
            "kir" => Some(Self::Kyrgyz),
            "tir" => Some(Self::Tigrinya),
            "am" => Some(Self::Amharic),
            "en-gb" => Some(Self::EnglishUK),
            "rup" => Some(Self::Aromanian),
            "es-cl" => Some(Self::SpanishChile),
            "an" => Some(Self::Aragonese),
            "skr" => Some(Self::Saraiki),
            "zh-hk" => Some(Self::ChineseHongKong),
            "kab" => Some(Self::Kabyle),
            "zh-sg" => Some(Self::ChineseSingapore),
            "kal" => Some(Self::Kalaallisut),
            "de_formal" => Some(Self::GermanFormal),
            "sr_latin" => Some(Self::SerbianLatin),
            "es-mx" => Some(Self::SpanishMexico),
            "de-ch" => Some(Self::GermanSwitzerland),
            "de-at" => Some(Self::GermanAustria),
            "nl-be" => Some(Self::DutchBelgium),
            _ => None,
        }
    }

    /// Returns the language slug (e.g., "en", "es", "pt-br") as a static string reference
    pub fn slug_str(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Afrikaans => "af",
            Self::Arabic => "ar",
            Self::Assamese => "as",
            Self::Belarusian => "bel",
            Self::Bulgarian => "bg",
            Self::Bambara => "bm",
            Self::Bengali => "bn",
            Self::Tibetan => "bo",
            Self::Catalan => "ca",
            Self::Czech => "cs",
            Self::Kashubian => "csb",
            Self::Welsh => "cy",
            Self::Danish => "da",
            Self::German => "de",
            Self::Dzongkha => "dz",
            Self::Greek => "el",
            Self::Esperanto => "eo",
            Self::Spanish => "es",
            Self::Estonian => "et",
            Self::Persian => "fa",
            Self::Finnish => "fi",
            Self::Faroese => "fo",
            Self::French => "fr",
            Self::Friulian => "fur",
            Self::WesternFrisian => "fy",
            Self::Irish => "ga",
            Self::Gujarati => "gu",
            Self::Hebrew => "he",
            Self::Hindi => "hi",
            Self::Hungarian => "hu",
            Self::Interlingua => "ia",
            Self::Indonesian => "id",
            Self::Icelandic => "is",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Georgian => "ka",
            Self::Khmer => "km",
            Self::Kannada => "kn",
            Self::Korean => "ko",
            Self::CentralKurdish => "ckb",
            Self::Latin => "la",
            Self::Limburgish => "li",
            Self::Lao => "lo",
            Self::Lithuanian => "lt",
            Self::Malayalam => "ml",
            Self::Malay => "ms",
            Self::LowGerman => "nds",
            Self::Dutch => "nl",
            Self::NorwegianNynorsk => "nn",
            Self::NorwegianBokmal => "nb",
            Self::OldNorse => "non",
            Self::Navajo => "nv",
            Self::Occitan => "oci",
            Self::Odia => "or",
            Self::Ossetic => "os",
            Self::Punjabi => "pa",
            Self::Polish => "pl",
            Self::Pashto => "ps",
            Self::Portuguese => "pt",
            Self::Romanian => "ro",
            Self::Russian => "ru",
            Self::Sardinian => "sc",
            Self::Slovak => "sk",
            Self::Slovenian => "sl",
            Self::Albanian => "sq",
            Self::Serbian => "sr",
            Self::Swedish => "sv",
            Self::Tamil => "ta",
            Self::Telugu => "te",
            Self::Thai => "th",
            Self::Tatar => "tt",
            Self::Ukrainian => "uk",
            Self::Urdu => "ur",
            Self::Walloon => "wa",
            Self::Yiddish => "yi",
            Self::Turkish => "tr",
            Self::Azerbaijani => "az",
            Self::Alemannic => "als",
            Self::Aramaic => "arc",
            Self::Asturian => "ast",
            Self::Avar => "av",
            Self::Aymara => "ay",
            Self::Bashkir => "ba",
            Self::Breton => "br",
            Self::Chechen => "ce",
            Self::Chuvash => "cv",
            Self::Dhivehi => "dv",
            Self::Basque => "eu",
            Self::Guarani => "gn",
            Self::Croatian => "hr",
            Self::NuosuYi => "ii",
            Self::Kashmiri => "ks",
            Self::Komi => "kv",
            Self::Macedonian => "mk",
            Self::Nahuatl => "nah",
            Self::Neapolitan => "nap",
            Self::PortugueseBrazil => "pt-br",
            Self::Quechua => "qu",
            Self::Sindhi => "snd",
            Self::Sundanese => "su",
            Self::Tahitian => "ty",
            Self::Udmurt => "udm",
            Self::Uyghur => "ug",
            Self::Venetian => "vec",
            Self::Vietnamese => "vi",
            Self::Kalmyk => "xal",
            Self::Zhuang => "za",
            Self::ChineseSimplified => "zh-cn",
            Self::ChineseTraditional => "zh-tw",
            Self::Latvian => "lv",
            Self::Bosnian => "bs",
            Self::Tagalog => "tl",
            Self::Nepali => "ne",
            Self::Galician => "gl",
            Self::Uzbek => "uz",
            Self::Somali => "so",
            Self::Marathi => "mr",
            Self::Kazakh => "kk",
            Self::Mirandese => "mwl",
            Self::Maltese => "mt",
            Self::Armenian => "hy",
            Self::GreekPolytonic => "el-po",
            Self::Iloko => "ilo",
            Self::Sinhala => "si",
            Self::Mongolian => "mn",
            Self::FrenchSwitzerland => "fr-ch",
            Self::FrenchCanada => "fr-ca",
            Self::ScottishGaelic => "gd",
            Self::Yoruba => "yo",
            Self::FrenchBelgium => "fr-be",
            Self::Kyrgyz => "kir",
            Self::Tigrinya => "tir",
            Self::Amharic => "am",
            Self::EnglishUK => "en-gb",
            Self::Aromanian => "rup",
            Self::SpanishChile => "es-cl",
            Self::Aragonese => "an",
            Self::Saraiki => "skr",
            Self::ChineseHongKong => "zh-hk",
            Self::Kabyle => "kab",
            Self::ChineseSingapore => "zh-sg",
            Self::Kalaallisut => "kal",
            Self::GermanFormal => "de_formal",
            Self::SerbianLatin => "sr_latin",
            Self::SpanishMexico => "es-mx",
            Self::GermanSwitzerland => "de-ch",
            Self::GermanAustria => "de-at",
            Self::DutchBelgium => "nl-be",
        }
    }

    /// Returns the display name in the native language as a static string reference
    pub fn name_str(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Afrikaans => "Afrikaans",
            Self::Arabic => "العربية",
            Self::Assamese => "অসমীয়া",
            Self::Belarusian => "Беларуская мова",
            Self::Bulgarian => "Български",
            Self::Bambara => "Bamanankan",
            Self::Bengali => "বাংলা",
            Self::Tibetan => "བོད་སྐད",
            Self::Catalan => "Català",
            Self::Czech => "Čeština",
            Self::Kashubian => "Kaszëbsczi",
            Self::Welsh => "Cymraeg",
            Self::Danish => "Dansk",
            Self::German => "Deutsch",
            Self::Dzongkha => "ཇོང་ཁ",
            Self::Greek => "Ελληνικά",
            Self::Esperanto => "Esperanto",
            Self::Spanish => "Español",
            Self::Estonian => "Eesti",
            Self::Persian => "فارسی",
            Self::Finnish => "Suomi",
            Self::Faroese => "Føroyskt",
            Self::French => "Français",
            Self::Friulian => "Friulian",
            Self::WesternFrisian => "Frysk",
            Self::Irish => "Gaelige",
            Self::Gujarati => "ગુજરાતી",
            Self::Hebrew => "עִבְרִית",
            Self::Hindi => "हिन्दी",
            Self::Hungarian => "Magyar",
            Self::Interlingua => "Interlingua",
            Self::Indonesian => "Bahasa Indonesia",
            Self::Icelandic => "Íslenska",
            Self::Italian => "Italiano",
            Self::Japanese => "日本語",
            Self::Georgian => "ქართული",
            Self::Khmer => "ភាសាខ្មែរ",
            Self::Kannada => "ಕನ್ನಡ",
            Self::Korean => "한국어",
            Self::CentralKurdish => "كوردی‎",
            Self::Latin => "Latine",
            Self::Limburgish => "Limburgs",
            Self::Lao => "ພາສາລາວ",
            Self::Lithuanian => "Lietuvių kalba",
            Self::Malayalam => "മലയാളം",
            Self::Malay => "Bahasa Melayu",
            Self::LowGerman => "Plattdüütsch",
            Self::Dutch => "Nederlands",
            Self::NorwegianNynorsk => "Norsk nynorsk",
            Self::NorwegianBokmal => "Norsk bokmål",
            Self::OldNorse => "Norrǿna",
            Self::Navajo => "Diné bizaad",
            Self::Occitan => "Occitan",
            Self::Odia => "ଓଡ଼ିଆ",
            Self::Ossetic => "Ирон",
            Self::Punjabi => "ਪੰਜਾਬੀ",
            Self::Polish => "Polski",
            Self::Pashto => "پښتو",
            Self::Portuguese => "Português",
            Self::Romanian => "Română",
            Self::Russian => "Русский",
            Self::Sardinian => "Sardu",
            Self::Slovak => "Slovenčina",
            Self::Slovenian => "Slovenščina",
            Self::Albanian => "Shqip",
            Self::Serbian => "Српски језик",
            Self::Swedish => "Svenska",
            Self::Tamil => "தமிழ்",
            Self::Telugu => "తెలుగు",
            Self::Thai => "ไทย",
            Self::Tatar => "Татар теле",
            Self::Ukrainian => "Українська",
            Self::Urdu => "اردو",
            Self::Walloon => "Walon",
            Self::Yiddish => "ייִדיש",
            Self::Turkish => "Türkçe",
            Self::Azerbaijani => "Azərbaycan dili",
            Self::Alemannic => "Alemannisch",
            Self::Aramaic => "ܕܥܒܪܸܝܛ",
            Self::Asturian => "Asturianu",
            Self::Avar => "авар мацӀ",
            Self::Aymara => "aymar aru",
            Self::Bashkir => "башҡорт теле",
            Self::Breton => "Brezhoneg",
            Self::Chechen => "Нохчийн мотт",
            Self::Chuvash => "чӑваш чӗлхи",
            Self::Dhivehi => "ދިވެހި",
            Self::Basque => "Euskara",
            Self::Guarani => "Avañe'ẽ",
            Self::Croatian => "Hrvatski",
            Self::NuosuYi => "ꆇꉙ",
            Self::Kashmiri => "कश्मीरी",
            Self::Komi => "Коми",
            Self::Macedonian => "Македонски јазик",
            Self::Nahuatl => "Nahuatl",
            Self::Neapolitan => "Nnapulitano",
            Self::PortugueseBrazil => "Português do Brasil",
            Self::Quechua => "Runa Simi",
            Self::Sindhi => "سنڌي",
            Self::Sundanese => "Basa Sunda",
            Self::Tahitian => "Reo Mā`ohi",
            Self::Udmurt => "Удмурт кыл",
            Self::Uyghur => "Uyƣurqə",
            Self::Venetian => "Vèneta",
            Self::Vietnamese => "Tiếng Việt",
            Self::Kalmyk => "Хальмг",
            Self::Zhuang => "(Cuengh)",
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
            Self::Latvian => "Latviešu valoda",
            Self::Bosnian => "Bosanski",
            Self::Tagalog => "Tagalog",
            Self::Nepali => "नेपाली",
            Self::Galician => "Galego",
            Self::Uzbek => "O‘zbekcha",
            Self::Somali => "Afsoomaali",
            Self::Marathi => "मराठी",
            Self::Kazakh => "Қазақ тілі",
            Self::Mirandese => "Mirandés",
            Self::Maltese => "Malti",
            Self::Armenian => "Հայերեն",
            Self::GreekPolytonic => "Greek (Polytonic)",
            Self::Iloko => "Pagsasao nga Iloko",
            Self::Sinhala => "සිංහල",
            Self::Mongolian => "Монгол",
            Self::FrenchSwitzerland => "Français de Suisse",
            Self::FrenchCanada => "Français du Canada",
            Self::ScottishGaelic => "Gàidhlig",
            Self::Yoruba => "èdè Yorùbá",
            Self::FrenchBelgium => "Français de Belgique",
            Self::Kyrgyz => "Кыргызча",
            Self::Tigrinya => "ትግርኛ",
            Self::Amharic => "አማርኛ",
            Self::EnglishUK => "English (UK)",
            Self::Aromanian => "Armãneashce",
            Self::SpanishChile => "Español de Chile",
            Self::Aragonese => "Aragonés",
            Self::Saraiki => "Saraiki",
            Self::ChineseHongKong => "香港中文版",
            Self::Kabyle => "Taqbaylit",
            Self::ChineseSingapore => "中文",
            Self::Kalaallisut => "Kalaallisut",
            Self::GermanFormal => "Deutsch (Sie)",
            Self::SerbianLatin => "Srpski (latinica)",
            Self::SpanishMexico => "Español de México",
            Self::GermanSwitzerland => "Deutsch (Schweiz)",
            Self::GermanAustria => "Deutsch (Österreich)",
            Self::DutchBelgium => "Nederlands (België)",
        }
    }

    /// Returns the WordPress locale code (e.g., "en_US", "es_ES") as a static string reference
    /// Returns an empty string if no WordPress locale is available
    pub fn wp_locale_str(&self) -> &'static str {
        match self {
            Self::English => "en_US",
            Self::Afrikaans => "af",
            Self::Arabic => "ar",
            Self::Assamese => "as",
            Self::Belarusian => "bel",
            Self::Bulgarian => "bg_BG",
            Self::Bambara => "",
            Self::Bengali => "bn_BD",
            Self::Tibetan => "bo",
            Self::Catalan => "ca",
            Self::Czech => "cs_CZ",
            Self::Kashubian => "",
            Self::Welsh => "cy",
            Self::Danish => "da_DK",
            Self::German => "de_DE",
            Self::Dzongkha => "",
            Self::Greek => "el",
            Self::Esperanto => "eo",
            Self::Spanish => "es_ES",
            Self::Estonian => "et",
            Self::Persian => "fa_IR",
            Self::Finnish => "fi",
            Self::Faroese => "fo",
            Self::French => "fr_FR",
            Self::Friulian => "fur",
            Self::WesternFrisian => "fy",
            Self::Irish => "ga",
            Self::Gujarati => "gu",
            Self::Hebrew => "he_IL",
            Self::Hindi => "hi_IN",
            Self::Hungarian => "hu_HU",
            Self::Interlingua => "",
            Self::Indonesian => "id_ID",
            Self::Icelandic => "is_IS",
            Self::Italian => "it_IT",
            Self::Japanese => "ja",
            Self::Georgian => "ka_GE",
            Self::Khmer => "km",
            Self::Kannada => "kn",
            Self::Korean => "ko_KR",
            Self::CentralKurdish => "ckb",
            Self::Latin => "",
            Self::Limburgish => "li",
            Self::Lao => "lo",
            Self::Lithuanian => "lt_LT",
            Self::Malayalam => "ml_IN",
            Self::Malay => "ms_MY",
            Self::LowGerman => "",
            Self::Dutch => "nl_NL",
            Self::NorwegianNynorsk => "nn_NO",
            Self::NorwegianBokmal => "nb_NO",
            Self::OldNorse => "",
            Self::Navajo => "",
            Self::Occitan => "oci",
            Self::Odia => "",
            Self::Ossetic => "os",
            Self::Punjabi => "pa_IN",
            Self::Polish => "pl_PL",
            Self::Pashto => "ps",
            Self::Portuguese => "pt_PT",
            Self::Romanian => "ro_RO",
            Self::Russian => "ru_RU",
            Self::Sardinian => "",
            Self::Slovak => "sk_SK",
            Self::Slovenian => "sl_SI",
            Self::Albanian => "sq",
            Self::Serbian => "sr_RS",
            Self::Swedish => "sv_SE",
            Self::Tamil => "ta_IN",
            Self::Telugu => "te",
            Self::Thai => "th",
            Self::Tatar => "tt_RU",
            Self::Ukrainian => "uk",
            Self::Urdu => "ur",
            Self::Walloon => "wa",
            Self::Yiddish => "",
            Self::Turkish => "tr_TR",
            Self::Azerbaijani => "az",
            Self::Alemannic => "",
            Self::Aramaic => "",
            Self::Asturian => "ast",
            Self::Avar => "",
            Self::Aymara => "",
            Self::Bashkir => "ba",
            Self::Breton => "bre",
            Self::Chechen => "",
            Self::Chuvash => "",
            Self::Dhivehi => "dv",
            Self::Basque => "eu",
            Self::Guarani => "gn",
            Self::Croatian => "hr",
            Self::NuosuYi => "",
            Self::Kashmiri => "",
            Self::Komi => "",
            Self::Macedonian => "mk_MK",
            Self::Nahuatl => "",
            Self::Neapolitan => "",
            Self::PortugueseBrazil => "pt_BR",
            Self::Quechua => "",
            Self::Sindhi => "snd",
            Self::Sundanese => "su_ID",
            Self::Tahitian => "",
            Self::Udmurt => "",
            Self::Uyghur => "ug_CN",
            Self::Venetian => "",
            Self::Vietnamese => "vi",
            Self::Kalmyk => "",
            Self::Zhuang => "",
            Self::ChineseSimplified => "zh_CN",
            Self::ChineseTraditional => "zh_TW",
            Self::Latvian => "lv",
            Self::Bosnian => "bs_BA",
            Self::Tagalog => "tl",
            Self::Nepali => "ne_NP",
            Self::Galician => "gl_ES",
            Self::Uzbek => "uz_UZ",
            Self::Somali => "so_SO",
            Self::Marathi => "mr",
            Self::Kazakh => "kk",
            Self::Mirandese => "",
            Self::Maltese => "",
            Self::Armenian => "hy",
            Self::GreekPolytonic => "el",
            Self::Iloko => "",
            Self::Sinhala => "si_LK",
            Self::Mongolian => "mn",
            Self::FrenchSwitzerland => "",
            Self::FrenchCanada => "fr_CA",
            Self::ScottishGaelic => "gd",
            Self::Yoruba => "",
            Self::FrenchBelgium => "fr_BE",
            Self::Kyrgyz => "kir",
            Self::Tigrinya => "tir",
            Self::Amharic => "am",
            Self::EnglishUK => "en_GB",
            Self::Aromanian => "rup_MK",
            Self::SpanishChile => "es_CL",
            Self::Aragonese => "",
            Self::Saraiki => "skr",
            Self::ChineseHongKong => "zh_HK",
            Self::Kabyle => "kab",
            Self::ChineseSingapore => "zh_SG",
            Self::Kalaallisut => "kal",
            Self::GermanFormal => "de",
            Self::SerbianLatin => "sr",
            Self::SpanishMexico => "es_MX",
            Self::GermanSwitzerland => "de_CH",
            Self::GermanAustria => "de_AT",
            Self::DutchBelgium => "nl_BE",
        }
    }

    /// Returns the UN M.49 territory codes associated with this language as a static slice
    pub fn territories_slice(&self) -> &'static [&'static str] {
        match self {
            Self::English => &["019"],
            Self::Afrikaans => &["002"],
            Self::Arabic => &["145", "002"],
            Self::Assamese => &["034"],
            Self::Belarusian => &["151"],
            Self::Bulgarian => &["151"],
            Self::Bambara => &["002"],
            Self::Bengali => &["034"],
            Self::Tibetan => &["030"],
            Self::Catalan => &["039"],
            Self::Czech => &["151"],
            Self::Kashubian => &["151"],
            Self::Welsh => &["154"],
            Self::Danish => &["154"],
            Self::German => &["155"],
            Self::Dzongkha => &["034"],
            Self::Greek => &["039"],
            Self::Esperanto => &[],
            Self::Spanish => &["019", "039"],
            Self::Estonian => &["154"],
            Self::Persian => &["034"],
            Self::Finnish => &["154"],
            Self::Faroese => &["154"],
            Self::French => &["155"],
            Self::Friulian => &["039"],
            Self::WesternFrisian => &["155"],
            Self::Irish => &["154"],
            Self::Gujarati => &["034"],
            Self::Hebrew => &["145"],
            Self::Hindi => &["034"],
            Self::Hungarian => &["151"],
            Self::Interlingua => &[],
            Self::Indonesian => &["035"],
            Self::Icelandic => &["154"],
            Self::Italian => &["039"],
            Self::Japanese => &["030"],
            Self::Georgian => &["145"],
            Self::Khmer => &["035"],
            Self::Kannada => &["034"],
            Self::Korean => &["030"],
            Self::CentralKurdish => &["145", "034"],
            Self::Latin => &["039"],
            Self::Limburgish => &["155"],
            Self::Lao => &["035"],
            Self::Lithuanian => &["154"],
            Self::Malayalam => &["034"],
            Self::Malay => &["035"],
            Self::LowGerman => &["155"],
            Self::Dutch => &["155"],
            Self::NorwegianNynorsk => &["154"],
            Self::NorwegianBokmal => &["154"],
            Self::OldNorse => &["154"],
            Self::Navajo => &["019"],
            Self::Occitan => &["155"],
            Self::Odia => &["034"],
            Self::Ossetic => &["145"],
            Self::Punjabi => &["034"],
            Self::Polish => &["151"],
            Self::Pashto => &["034"],
            Self::Portuguese => &["039"],
            Self::Romanian => &["151"],
            Self::Russian => &["151"],
            Self::Sardinian => &["039"],
            Self::Slovak => &["151"],
            Self::Slovenian => &["039"],
            Self::Albanian => &["039"],
            Self::Serbian => &["039"],
            Self::Swedish => &["154"],
            Self::Tamil => &["034", "035"],
            Self::Telugu => &["034"],
            Self::Thai => &["035"],
            Self::Tatar => &["151"],
            Self::Ukrainian => &["151"],
            Self::Urdu => &["034"],
            Self::Walloon => &["155"],
            Self::Yiddish => &[],
            Self::Turkish => &["145"],
            Self::Azerbaijani => &["145"],
            Self::Alemannic => &["155"],
            Self::Aramaic => &["145"],
            Self::Asturian => &["039"],
            Self::Avar => &["151"],
            Self::Aymara => &["019"],
            Self::Bashkir => &["151"],
            Self::Breton => &["155"],
            Self::Chechen => &["151"],
            Self::Chuvash => &["151"],
            Self::Dhivehi => &["034"],
            Self::Basque => &["039"],
            Self::Guarani => &["019"],
            Self::Croatian => &["039"],
            Self::NuosuYi => &["030"],
            Self::Kashmiri => &["034"],
            Self::Komi => &["151"],
            Self::Macedonian => &["039"],
            Self::Nahuatl => &["019"],
            Self::Neapolitan => &["039"],
            Self::PortugueseBrazil => &["019"],
            Self::Quechua => &["019"],
            Self::Sindhi => &["034"],
            Self::Sundanese => &["035"],
            Self::Tahitian => &["009"],
            Self::Udmurt => &["151"],
            Self::Uyghur => &["030"],
            Self::Venetian => &["039"],
            Self::Vietnamese => &["035"],
            Self::Kalmyk => &["143"],
            Self::Zhuang => &["030"],
            Self::ChineseSimplified => &["030"],
            Self::ChineseTraditional => &["030"],
            Self::Latvian => &["154"],
            Self::Bosnian => &["039"],
            Self::Tagalog => &["035"],
            Self::Nepali => &["034"],
            Self::Galician => &["039"],
            Self::Uzbek => &["143"],
            Self::Somali => &["002"],
            Self::Marathi => &["034"],
            Self::Kazakh => &["143"],
            Self::Mirandese => &["039"],
            Self::Maltese => &["039"],
            Self::Armenian => &["145"],
            Self::GreekPolytonic => &["039"],
            Self::Iloko => &["035"],
            Self::Sinhala => &["034"],
            Self::Mongolian => &["030"],
            Self::FrenchSwitzerland => &["155"],
            Self::FrenchCanada => &["019"],
            Self::ScottishGaelic => &["154"],
            Self::Yoruba => &["002"],
            Self::FrenchBelgium => &["155"],
            Self::Kyrgyz => &["143"],
            Self::Tigrinya => &["002"],
            Self::Amharic => &["002"],
            Self::EnglishUK => &["154"],
            Self::Aromanian => &["151"],
            Self::SpanishChile => &["019"],
            Self::Aragonese => &[],
            Self::Saraiki => &["034"],
            Self::ChineseHongKong => &["030"],
            Self::Kabyle => &["002"],
            Self::ChineseSingapore => &["035"],
            Self::Kalaallisut => &["154"],
            Self::GermanFormal => &["155"],
            Self::SerbianLatin => &["039"],
            Self::SpanishMexico => &["019"],
            Self::GermanSwitzerland => &["155"],
            Self::GermanAustria => &["155"],
            Self::DutchBelgium => &["155"],
        }
    }

    /// Returns the popularity rank if this is a popular language (internal method)
    /// Returns None if the language is not marked as popular
    fn popular_rank_internal(&self) -> Option<u8> {
        match self {
            Self::English => Some(1),
            Self::Spanish => Some(2),
            Self::PortugueseBrazil => Some(3),
            Self::German => Some(4),
            Self::French => Some(5),
            Self::Hebrew => Some(6),
            Self::Japanese => Some(7),
            Self::Italian => Some(8),
            Self::Dutch => Some(9),
            Self::Russian => Some(10),
            Self::Turkish => Some(11),
            Self::Indonesian => Some(12),
            Self::ChineseSimplified => Some(13),
            Self::ChineseTraditional => Some(14),
            Self::Korean => Some(15),
            Self::Arabic => Some(16),
            Self::Swedish => Some(17),
            _ => None,
        }
    }

    /// Returns the parent locale slug if this is a variant as a static string reference
    pub fn parent_locale_slug_str(&self) -> Option<&'static str> {
        match self {
            Self::FrenchSwitzerland => Some("fr"),
            Self::FrenchCanada => Some("fr"),
            Self::FrenchBelgium => Some("fr"),
            Self::SpanishChile => Some("es"),
            Self::ChineseSingapore => Some("zh-cn"),
            Self::GermanFormal => Some("de"),
            Self::SerbianLatin => Some("sr"),
            Self::SpanishMexico => Some("es"),
            Self::GermanSwitzerland => Some("de"),
            Self::GermanAustria => Some("de"),
            Self::DutchBelgium => Some("nl"),
            _ => None,
        }
    }

    /// Returns all available languages
    pub fn all() -> Vec<Self> {
        vec![
            Self::English,
            Self::Afrikaans,
            Self::Arabic,
            Self::Assamese,
            Self::Belarusian,
            Self::Bulgarian,
            Self::Bambara,
            Self::Bengali,
            Self::Tibetan,
            Self::Catalan,
            Self::Czech,
            Self::Kashubian,
            Self::Welsh,
            Self::Danish,
            Self::German,
            Self::Dzongkha,
            Self::Greek,
            Self::Esperanto,
            Self::Spanish,
            Self::Estonian,
            Self::Persian,
            Self::Finnish,
            Self::Faroese,
            Self::French,
            Self::Friulian,
            Self::WesternFrisian,
            Self::Irish,
            Self::Gujarati,
            Self::Hebrew,
            Self::Hindi,
            Self::Hungarian,
            Self::Interlingua,
            Self::Indonesian,
            Self::Icelandic,
            Self::Italian,
            Self::Japanese,
            Self::Georgian,
            Self::Khmer,
            Self::Kannada,
            Self::Korean,
            Self::CentralKurdish,
            Self::Latin,
            Self::Limburgish,
            Self::Lao,
            Self::Lithuanian,
            Self::Malayalam,
            Self::Malay,
            Self::LowGerman,
            Self::Dutch,
            Self::NorwegianNynorsk,
            Self::NorwegianBokmal,
            Self::OldNorse,
            Self::Navajo,
            Self::Occitan,
            Self::Odia,
            Self::Ossetic,
            Self::Punjabi,
            Self::Polish,
            Self::Pashto,
            Self::Portuguese,
            Self::Romanian,
            Self::Russian,
            Self::Sardinian,
            Self::Slovak,
            Self::Slovenian,
            Self::Albanian,
            Self::Serbian,
            Self::Swedish,
            Self::Tamil,
            Self::Telugu,
            Self::Thai,
            Self::Tatar,
            Self::Ukrainian,
            Self::Urdu,
            Self::Walloon,
            Self::Yiddish,
            Self::Turkish,
            Self::Azerbaijani,
            Self::Alemannic,
            Self::Aramaic,
            Self::Asturian,
            Self::Avar,
            Self::Aymara,
            Self::Bashkir,
            Self::Breton,
            Self::Chechen,
            Self::Chuvash,
            Self::Dhivehi,
            Self::Basque,
            Self::Guarani,
            Self::Croatian,
            Self::NuosuYi,
            Self::Kashmiri,
            Self::Komi,
            Self::Macedonian,
            Self::Nahuatl,
            Self::Neapolitan,
            Self::PortugueseBrazil,
            Self::Quechua,
            Self::Sindhi,
            Self::Sundanese,
            Self::Tahitian,
            Self::Udmurt,
            Self::Uyghur,
            Self::Venetian,
            Self::Vietnamese,
            Self::Kalmyk,
            Self::Zhuang,
            Self::ChineseSimplified,
            Self::ChineseTraditional,
            Self::Latvian,
            Self::Bosnian,
            Self::Tagalog,
            Self::Nepali,
            Self::Galician,
            Self::Uzbek,
            Self::Somali,
            Self::Marathi,
            Self::Kazakh,
            Self::Mirandese,
            Self::Maltese,
            Self::Armenian,
            Self::GreekPolytonic,
            Self::Iloko,
            Self::Sinhala,
            Self::Mongolian,
            Self::FrenchSwitzerland,
            Self::FrenchCanada,
            Self::ScottishGaelic,
            Self::Yoruba,
            Self::FrenchBelgium,
            Self::Kyrgyz,
            Self::Tigrinya,
            Self::Amharic,
            Self::EnglishUK,
            Self::Aromanian,
            Self::SpanishChile,
            Self::Aragonese,
            Self::Saraiki,
            Self::ChineseHongKong,
            Self::Kabyle,
            Self::ChineseSingapore,
            Self::Kalaallisut,
            Self::GermanFormal,
            Self::SerbianLatin,
            Self::SpanishMexico,
            Self::GermanSwitzerland,
            Self::GermanAustria,
            Self::DutchBelgium,
        ]
    }

    /// Returns only the popular languages, sorted by popularity rank
    pub fn popular() -> Vec<Self> {
        let mut languages = Self::all();
        languages.retain(|lang| lang.popular_rank().is_some());
        languages.sort_by_key(|lang| lang.popular_rank());
        languages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_id_matches_discriminant() {
        assert_eq!(WPComLanguage::English.language_id(), 1);
        assert_eq!(WPComLanguage::Spanish.language_id(), 19);
        assert_eq!(WPComLanguage::PortugueseBrazil.language_id(), 438);
        assert_eq!(WPComLanguage::GermanFormal.language_id(), 900);
    }

    #[test]
    fn test_language_metadata() {
        let english = WPComLanguage::English;
        assert_eq!(english.slug(), "en");
        assert_eq!(english.name(), "English");
        assert_eq!(english.wp_locale(), "en_US");
        assert_eq!(english.territories(), vec!["019"]);
        assert_eq!(english.popular_rank(), Some(1));
        assert_eq!(english.parent_locale_slug(), None);
    }

    #[test]
    fn test_variant_with_parent() {
        let german_formal = WPComLanguage::GermanFormal;
        assert_eq!(german_formal.slug(), "de_formal");
        assert_eq!(german_formal.parent_locale_slug(), Some("de".to_string()));
    }

    #[test]
    fn test_popular_languages() {
        let popular = WPComLanguage::popular();
        assert_eq!(popular.len(), 17);
        assert_eq!(popular[0], WPComLanguage::English);
        assert_eq!(popular[1], WPComLanguage::Spanish);
    }

    #[test]
    fn test_from_discriminant() {
        // Test valid IDs
        assert_eq!(
            WPComLanguage::from_discriminant(1),
            Some(WPComLanguage::English)
        );
        assert_eq!(
            WPComLanguage::from_discriminant(19),
            Some(WPComLanguage::Spanish)
        );
        assert_eq!(
            WPComLanguage::from_discriminant(438),
            Some(WPComLanguage::PortugueseBrazil)
        );
        assert_eq!(
            WPComLanguage::from_discriminant(900),
            Some(WPComLanguage::GermanFormal)
        );

        // Test invalid IDs
        assert_eq!(WPComLanguage::from_discriminant(0), None);
        assert_eq!(WPComLanguage::from_discriminant(77), None); // Gap in IDs
        assert_eq!(WPComLanguage::from_discriminant(999), None);
        assert_eq!(WPComLanguage::from_discriminant(1000), None);
    }

    #[test]
    fn test_from_id() {
        // from_id should behave the same as from_discriminant
        assert_eq!(WPComLanguage::from_id(1), Some(WPComLanguage::English));
        assert_eq!(WPComLanguage::from_id(24), Some(WPComLanguage::French));
        assert_eq!(WPComLanguage::from_id(0), None);
    }

    #[test]
    fn test_from_slug() {
        // Test valid slugs
        assert_eq!(WPComLanguage::from_slug("en"), Some(WPComLanguage::English));
        assert_eq!(WPComLanguage::from_slug("es"), Some(WPComLanguage::Spanish));
        assert_eq!(
            WPComLanguage::from_slug("pt-br"),
            Some(WPComLanguage::PortugueseBrazil)
        );
        assert_eq!(
            WPComLanguage::from_slug("de_formal"),
            Some(WPComLanguage::GermanFormal)
        );
        assert_eq!(
            WPComLanguage::from_slug("zh-cn"),
            Some(WPComLanguage::ChineseSimplified)
        );
        assert_eq!(
            WPComLanguage::from_slug("zh-tw"),
            Some(WPComLanguage::ChineseTraditional)
        );

        // Test invalid slugs
        assert_eq!(WPComLanguage::from_slug(""), None);
        assert_eq!(WPComLanguage::from_slug("invalid"), None);
        assert_eq!(WPComLanguage::from_slug("EN"), None); // Case sensitive
    }

    #[test]
    fn test_roundtrip_discriminant() {
        // Test that all languages can be looked up by their discriminant
        for lang in WPComLanguage::all() {
            let id = lang.language_id();
            assert_eq!(
                WPComLanguage::from_discriminant(id),
                Some(lang),
                "Failed to lookup language by discriminant: {:?} (id: {})",
                lang,
                id
            );
        }
    }

    #[test]
    fn test_roundtrip_slug() {
        // Test that all languages can be looked up by their slug
        for lang in WPComLanguage::all() {
            let slug = lang.slug();
            assert_eq!(
                WPComLanguage::from_slug(&slug),
                Some(lang),
                "Failed to lookup language by slug: {:?} (slug: {})",
                lang,
                slug
            );
        }
    }

    #[test]
    fn test_lookup_consistency() {
        // Verify that from_id and from_discriminant are equivalent
        for id in [1, 19, 24, 438, 900, 0, 77, 999] {
            assert_eq!(
                WPComLanguage::from_id(id),
                WPComLanguage::from_discriminant(id),
                "from_id and from_discriminant differ for id: {}",
                id
            );
        }
    }

    #[test]
    fn test_languages_get_params_with_english_locale() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/wpcom/v2/i18n/language-names")
                .expect("Failed to parse url");

        let params = LanguagesGetParams {
            locale: Some(WPComLanguage::English),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/i18n/language-names?_locale=en"
        );
    }

    #[test]
    fn test_languages_get_params_with_portuguese_brazil_locale() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/wpcom/v2/i18n/language-names")
                .expect("Failed to parse url");

        let params = LanguagesGetParams {
            locale: Some(WPComLanguage::PortugueseBrazil),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/i18n/language-names?_locale=pt-br"
        );
    }
}
