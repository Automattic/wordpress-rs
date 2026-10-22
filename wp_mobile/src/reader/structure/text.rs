//! Text, as placeholder words: the same number of them, in the same scripts, with the same
//! punctuation, emoji and spaces around them, so reading time and text direction come out the same.
//!
//! Each word becomes one of a few dozen stand-ins, picked by a hash of the word and the post. So the
//! same word maps the same way everywhere in a post, and an excerpt still matches the post's start,
//! but many words share each stand-in, and the choice differs between posts, so the stand-ins can't
//! be read back.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::attribute::{Decimal, Length};
use super::url::{Url, is_small_number};
use super::vocabulary::{GettyKey, GettyTld, Lorem, Phrase, ShortcodeKey, ShortcodeName};
use super::{Anchor, Names, UserId, Vocabulary};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Text(pub(crate) Vec<Token>);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Token {
    /// A word in Latin script, or one no other stand-in has the letters for.
    Lorem(Lorem),
    /// The same, starting with a capital.
    Capital(Lorem),
    /// A word that starts with a digit, like `3d`, which a shortcode's name can't.
    Digit(Lorem),
    /// A word in another script, with as many letters.
    Script {
        script: Script,
        letters: u16,
        stand_in: u8,
        #[serde(default)]
        capital: bool,
    },
    /// A number, by how many digits it has, up to four.
    Number(u8),
    /// A Chinese or Japanese character, which don't put spaces between words.
    Unspaced {
        kana: bool,
        stand_in: u8,
    },
    /// The word after an `@`.
    User(UserId),
    Space,
    Separator(Separator),
    /// Something WordPress or Jetpack wrote, like "Continue reading".
    Phrase(Phrase),
    Url(Url),
    Shortcode(Shortcode),
}

/// What's between words: punctuation, emoji and spaces, but never a letter or a digit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Separator(String);

impl TryFrom<String> for Separator {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.is_empty() || text.chars().any(char::is_alphanumeric) {
            Err(format!("{text:?} isn't punctuation"))
        } else {
            Ok(Separator(text))
        }
    }
}

impl From<Separator> for String {
    fn from(separator: Separator) -> Self {
        separator.0
    }
}

/// The scripts with letters of their own to stand in with, so text keeps its direction and script.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Script {
    Arabic,
    Hebrew,
    Cyrillic,
    Greek,
    Devanagari,
    Thai,
    Hiragana,
    Katakana,
    Hangul,
    Han,
}

const SCRIPTS: &[(Script, u32, u32, &str)] = &[
    (
        Script::Arabic,
        0x0600,
        0x06FF,
        "ابتثجحخدذرزسشصضطظعغفقكلمنهوي",
    ),
    (Script::Hebrew, 0x0590, 0x05FF, "אבגדהוזחטיכלמנסעפצקרשת"),
    (
        Script::Cyrillic,
        0x0400,
        0x04FF,
        "абвгдежзиклмнопрстуфхцчшэюя",
    ),
    (Script::Greek, 0x0370, 0x03FF, "αβγδεζηθικλμνξοπρστυφχψω"),
    (
        Script::Devanagari,
        0x0900,
        0x097F,
        "कखगघचछजझटठडढतथदधनपफबभमयरलवशसह",
    ),
    (
        Script::Thai,
        0x0E00,
        0x0E7F,
        "กขคงจฉชซญดตถทธนบปผพฟภมยรลวศสหอฮ",
    ),
    (
        Script::Hiragana,
        0x3040,
        0x309F,
        "あいうえおかきくけこさしすせそたちつてとなにぬねの",
    ),
    (
        Script::Katakana,
        0x30A0,
        0x30FF,
        "アイウエオカキクケコサシスセソタチツテトナニヌネノ",
    ),
    (
        Script::Hangul,
        0xAC00,
        0xD7AF,
        "가나다라마바사아자차카타파하",
    ),
    (
        Script::Han,
        0x4E00,
        0x9FFF,
        "的一是不了人我在有他这中大来上国个到说们为子和你地出道也时年得就那要下以生会自着去之过家学对可",
    ),
];

/// How many stand-ins each script's words share.
const STAND_INS: u64 = 34;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Shortcode {
    #[serde(default)]
    pub(crate) closing: bool,
    pub(crate) name: ShortcodeName,
    #[serde(default)]
    pub(crate) arguments: Vec<Argument>,
    #[serde(default)]
    pub(crate) self_closing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Argument {
    /// `None` for an argument without a name, like `[audio https://…]`'s.
    #[serde(default)]
    pub(crate) key: Option<ShortcodeKey>,
    pub(crate) value: ArgumentValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArgumentValue {
    Url(Url),
    Number(Decimal),
    /// A gallery's attachment IDs, by how many there are.
    Ids(u16),
    Other,
}

// --- Encoding -------------------------------------------------------------------------------------

impl Text {
    pub(crate) fn encode(names: &mut Names, text: &str) -> Text {
        let mut tokens = Vec::new();
        if let Some(phrase) = Phrase::parse(text.trim()) {
            let start = text.len() - text.trim_start().len();
            let end = text.trim_end().len();
            push_separator(&mut tokens, &text[..start]);
            tokens.push(Token::Phrase(phrase));
            push_separator(&mut tokens, &text[end..]);
            return Text(tokens);
        }
        // A URL stays a URL, since the cleanup turns an unresolved embed's into a link.
        let mut rest = text;
        while let Some(found) = URL_ANYWHERE.find(rest) {
            shortcodes(names, &rest[..found.start()], &mut tokens);
            tokens.push(Token::Url(Url::encode(names, found.as_str())));
            rest = &rest[found.end()..];
        }
        shortcodes(names, rest, &mut tokens);
        Text(tokens)
    }

    pub(crate) fn render(&self) -> String {
        let mut text = String::new();
        for token in &self.0 {
            match token {
                Token::Lorem(word) => text += word.as_str(),
                Token::Capital(word) => text += &capitalized(word.as_str()),
                Token::Digit(word) => {
                    text.push('0');
                    text += word.as_str();
                }
                Token::Script {
                    script,
                    letters,
                    stand_in,
                    capital,
                } => {
                    let alphabet: Vec<char> = SCRIPTS
                        .iter()
                        .find(|(found, ..)| found == script)
                        .map(|(.., letters)| letters.chars().collect())
                        .unwrap_or_default();
                    let word: String = (0..*letters)
                        .map(|index| {
                            let hash = fnv(&[&[*stand_in], &index.to_le_bytes()]);
                            alphabet[(hash % alphabet.len() as u64) as usize]
                        })
                        .collect();
                    text += &if *capital { capitalized(&word) } else { word };
                }
                Token::Number(digits) => text += &"0".repeat(usize::from(*digits)),
                Token::Unspaced { kana, stand_in } => {
                    let letters = if *kana {
                        "あいうえおかきく"
                    } else {
                        "的一是不了人我在"
                    };
                    text.extend(letters.chars().nth(usize::from(*stand_in) % 8));
                }
                Token::User(user) => text += &format!("user{}", user.0),
                Token::Space => text.push(' '),
                Token::Separator(separator) => text += &separator.0,
                Token::Phrase(phrase) => text += phrase.as_str(),
                Token::Url(url) => text += &url.render(),
                Token::Shortcode(shortcode) => text += &shortcode.render(),
            }
        }
        text
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn push_separator(tokens: &mut Vec<Token>, text: &str) {
    if text.is_empty() {
        return;
    }
    if text == " " {
        tokens.push(Token::Space);
    } else if let Ok(separator) = Separator::try_from(text.to_owned()) {
        tokens.push(Token::Separator(separator));
    }
}

/// Text with its shortcodes, which keep their name, and the URL an embed's points at.
fn shortcodes(names: &mut Names, text: &str, tokens: &mut Vec<Token>) {
    let mut rest = text;
    while let Some(found) = SHORTCODE.find(rest) {
        words(names, &rest[..found.start()], tokens);
        tokens.push(Token::Shortcode(Shortcode::encode(names, found.as_str())));
        rest = &rest[found.end()..];
    }
    words(names, rest, tokens);
}

fn words(names: &mut Names, text: &str, tokens: &mut Vec<Token>) {
    let mut word = String::new();
    let mut separator = String::new();
    // A word after an `@` is a username, which maps like a mention's `data-username`. P2 puts the
    // `@` in an element of its own, so it carries over to the next text.
    let flush_word = |word: &mut String, tokens: &mut Vec<Token>, names: &mut Names| {
        if !word.is_empty() {
            tokens.push(if names.after_at {
                Token::User(UserId::of(names, &word.to_ascii_lowercase()))
            } else {
                placeholder(names, word)
            });
            word.clear();
            names.after_at = false;
        }
    };
    let flush_separator = |separator: &mut String, tokens: &mut Vec<Token>| {
        push_separator(tokens, separator);
        separator.clear();
    };
    for c in text.chars() {
        if is_unspaced(c) {
            // Where a word ends depends on the markup, so each character maps on its own, to one of
            // a few, which keeps how many there are, but not which.
            flush_word(&mut word, tokens, names);
            flush_separator(&mut separator, tokens);
            names.after_at = false;
            let mut bytes = [0; 4];
            let hash = fnv(&[names.salt.as_bytes(), c.encode_utf8(&mut bytes).as_bytes()]);
            tokens.push(Token::Unspaced {
                kana: ('\u{3040}'..='\u{30FF}').contains(&c),
                stand_in: (hash % 8) as u8,
            });
        } else if c.is_alphanumeric() || (c == '_' && !word.is_empty()) {
            flush_separator(&mut separator, tokens);
            word.push(c);
        } else {
            flush_word(&mut word, tokens, names);
            names.after_at = c == '@';
            separator.push(c);
        }
    }
    flush_word(&mut word, tokens, names);
    flush_separator(&mut separator, tokens);
}

/// The stand-in for `word`. It keeps the word's script, and whether it's a number or starts with a
/// capital.
fn placeholder(names: &Names, word: &str) -> Token {
    let first = word.chars().next().expect("words aren't empty");
    if word.chars().all(|c| c.is_ascii_digit()) {
        return Token::Number(word.len().min(4) as u8);
    }
    let lower = word.to_lowercase();
    let stand_in = fnv(&[
        names.salt.as_bytes(),
        lower.as_bytes(),
        &0usize.to_le_bytes(),
    ]) % STAND_INS;
    let capital = first.is_uppercase();
    match SCRIPTS
        .iter()
        .find(|(_, start, end, _)| (*start..=*end).contains(&(first as u32)))
    {
        Some((script, ..)) => Token::Script {
            script: *script,
            letters: word.chars().count().min(usize::from(u16::MAX)) as u16,
            stand_in: stand_in as u8,
            capital,
        },
        None => {
            let lorem = Lorem::ALL[stand_in as usize % Lorem::ALL.len()];
            if first.is_ascii_digit() {
                Token::Digit(lorem)
            } else if capital {
                Token::Capital(lorem)
            } else {
                Token::Lorem(lorem)
            }
        }
    }
}

/// Whether a character is Chinese or Japanese, which don't put spaces between words. The same
/// ranges as the reading time's.
fn is_unspaced(c: char) -> bool {
    matches!(
        c,
        '\u{3040}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
}

/// FNV-1a, which gives the same stand-ins on every computer and toolchain.
pub(crate) fn fnv(parts: &[&[u8]]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.iter().chain(&[0]) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

impl Shortcode {
    fn encode(names: &mut Names, shortcode: &str) -> Shortcode {
        let captures = SHORTCODE.captures(shortcode).expect("it matched");
        let known = ShortcodeName::parse(&captures[2]);
        let arguments = SHORTCODE_ARGUMENT
            .captures_iter(&captures[3])
            .map(|argument| {
                let value = argument.get(3).or(argument.get(4)).or(argument.get(5));
                let value = value.map_or("", |value| value.as_str());
                let value = if URL_IN_TEXT.is_match(value) {
                    ArgumentValue::Url(Url::encode(names, value))
                } else if let Some(number) = is_small_number(value)
                    .then(|| Decimal::new(value))
                    .flatten()
                {
                    ArgumentValue::Number(number)
                } else if !value.is_empty()
                    && value
                        .split(',')
                        .all(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
                {
                    ArgumentValue::Ids(value.split(',').count().min(usize::from(u16::MAX)) as u16)
                } else {
                    ArgumentValue::Other
                };
                let key = argument.get(2).map(|key| match known {
                    Some(_) => {
                        ShortcodeKey::parse_or_other(key.as_str()).expect("it has a stand-in")
                    }
                    None => ShortcodeKey::other().expect("it has a stand-in"),
                });
                Argument { key, value }
            })
            .collect();
        Shortcode {
            closing: !captures[1].is_empty(),
            name: known.unwrap_or(ShortcodeName::Custom),
            arguments,
            self_closing: !captures[4].is_empty(),
        }
    }

    fn render(&self) -> String {
        let arguments: String = self
            .arguments
            .iter()
            .map(|argument| {
                let value = match &argument.value {
                    ArgumentValue::Url(url) => url.render(),
                    ArgumentValue::Number(number) => number.as_str().to_owned(),
                    ArgumentValue::Ids(count) => vec!["0"; usize::from(*count)].join(","),
                    ArgumentValue::Other => "x".to_owned(),
                };
                match argument.key {
                    Some(key) => format!(r#" {}="{value}""#, key.as_str()),
                    None => format!(" {value}"),
                }
            })
            .collect();
        format!(
            "[{}{}{arguments}{}]",
            if self.closing { "/" } else { "" },
            self.name.as_str(),
            if self.self_closing { "/" } else { "" }
        )
    }
}

static URL_ANYWHERE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(https?:)?//[^\s<>"'\[\]]+"#).expect("a valid regex"));
static URL_IN_TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(https?:)?//\S+$").expect("a valid regex"));
static SHORTCODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[(/?)([a-z][a-z0-9_\-]*)((?:\s+(?:[a-zA-Z_\-]+=(?:"[^"]*"|'[^']*'|[^\s\]"']+)|"[^"]*"|[^\s\]"'=]+))*)\s*(/?)\]"#)
        .expect("a valid regex")
});
static SHORTCODE_ARGUMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\s+(([a-zA-Z_\-]+)=)?(?:"([^"]*)"|'([^']*)'|([^\s\]"']+))"#)
        .expect("a valid regex")
});

// --- Getty ----------------------------------------------------------------------------------------

/// One of Getty's `gie.widgets.load({…})` calls.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct GettyLoad(Vec<GettyField>);

impl GettyLoad {
    /// The embed's ID, which matches its link's.
    pub(crate) fn ids(&self) -> impl Iterator<Item = Anchor> + '_ {
        self.0.iter().filter_map(|field| match field.value {
            GettyValue::Id(id) => Some(id),
            _ => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct GettyField {
    key: GettyKey,
    value: GettyValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GettyValue {
    /// The embed's ID, which matches its link's.
    Id(Anchor),
    Size(Length),
    Tld(GettyTld),
    /// The image's number.
    Items,
    Flag(bool),
    Other,
}

/// Getty's `gie.widgets.load({…})` calls, with stand-ins for the embed's ID, which matches its
/// link's, its signature and its image's number. Empty for any other script.
pub(crate) fn getty(names: &mut Names, code: &str) -> Vec<GettyLoad> {
    GETTY_LOAD
        .captures_iter(code)
        .map(|load| {
            GettyLoad(
                GETTY_FIELD
                    .captures_iter(&load[1])
                    .filter_map(|field| {
                        let key = GettyKey::parse(&field[1])?;
                        let text = field
                            .get(2)
                            .or(field.get(3))
                            .map_or("", |value| value.as_str());
                        let value =
                            if let Some(flag) = field.get(4) {
                                GettyValue::Flag(flag.as_str() == "true")
                            } else {
                                match key {
                                    GettyKey::Id => GettyValue::Id(Anchor::of(names, text)),
                                    GettyKey::W | GettyKey::H => Length::parse(text)
                                        .map_or(GettyValue::Other, GettyValue::Size),
                                    GettyKey::Tld => GettyTld::parse(text)
                                        .map_or(GettyValue::Other, GettyValue::Tld),
                                    GettyKey::Items => GettyValue::Items,
                                    _ => GettyValue::Other,
                                }
                            };
                        Some(GettyField { key, value })
                    })
                    .collect(),
            )
        })
        .collect()
}

pub(crate) fn render_getty(loads: &[GettyLoad]) -> String {
    loads
        .iter()
        .map(|load| {
            let fields: Vec<String> = load
                .0
                .iter()
                .map(|field| {
                    let value = match &field.value {
                        GettyValue::Id(id) => format!("'{}'", id.render()),
                        GettyValue::Size(size) => format!("'{}'", size.render()),
                        GettyValue::Tld(tld) => format!("'{}'", tld.as_str()),
                        GettyValue::Items => "'0'".to_owned(),
                        GettyValue::Flag(flag) => flag.to_string(),
                        GettyValue::Other => "'x'".to_owned(),
                    };
                    format!("{}:{value}", field.key.as_str())
                })
                .collect();
            format!(
                "gie(function(){{gie.widgets.load({{{}}})}});",
                fields.join(",")
            )
        })
        .collect()
}

static GETTY_LOAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"gie\.widgets\.load\(\{(.*?)\}\)").expect("a valid regex"));
static GETTY_FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(\w+)\s*:\s*(?:'([^']*)'|"([^"]*)"|(true|false))"#).expect("a valid regex")
});
