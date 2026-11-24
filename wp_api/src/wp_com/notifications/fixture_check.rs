//! Fails when a notification fixture or hand-built test input in the repository shares a value
//! with a folder of recorded notes kept outside the repository. Recorded notes must never reach
//! the repository, so every fixture is written by hand; this check catches one that wasn't.
//!
//! Run it with the same folders as the corpus test, before pushing notification work:
//!
//! ```bash
//! NOTIFICATIONS_CORPUS=/path/to/notes cargo test -p wp_api --lib \
//!     wp_com::notifications::fixture_check -- --ignored --nocapture
//! ```
//!
//! It reads every file Git would commit whose path contains `notification` or `styled_text`, and
//! every file in the folders named in `NOTIFICATIONS_FIXTURE_CHECK_EXTRA` (separated as in
//! `PATH`). From them it takes these tokens: numbers of six or more digits, runs of twelve or more
//! hex characters, the host of each URL, and words of seven or more letters.
//!
//! A token that's also in a recorded note is shared, unless it's part of the API's structure or
//! of ordinary language:
//!
//! - it's in more than [`STRUCTURE_NOTES`] notes: a recorded value belongs to one note or a few,
//!   while the values every note of a kind carries repeat across thousands;
//! - it's a word in an object key or in a `type` value, which are names the API defines;
//! - it's a word the repository also uses outside the notification files. Recorded text is
//!   scrambled into random letters, so a real word in a note is a URL path, a markup tag or other
//!   text that was never private, and a word the rest of the code base uses wasn't copied.
//!
//! The check fails when there's nothing to compare against, so a missing folder never reads as a
//! pass. It never prints anything from a note except a token that's also in a repository file.

use super::corpus::note_files;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

/// A token in more notes than this is part of the API's structure, not a value from one note.
const STRUCTURE_NOTES: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TokenKind {
    Number,
    Hash,
    Host,
    Word,
}

impl TokenKind {
    fn name(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Hash => "hash",
            Self::Host => "host",
            Self::Word => "word",
        }
    }
}

/// Calls `found` with every token in `text`. Hashes, hosts and words are lowercased.
fn tokens(text: &str, found: &mut impl FnMut(TokenKind, &str)) {
    let mut buffer = String::new();
    let bytes = text.as_bytes();
    let end = [(bytes.len(), &b' ')];

    // Numbers: runs of six or more digits, wherever they are.
    let mut start = None;
    for (index, byte) in bytes.iter().enumerate().chain(end) {
        match (byte.is_ascii_digit(), start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                if index - from >= 6 {
                    found(TokenKind::Number, &text[from..index]);
                }
                start = None;
            }
            _ => {}
        }
    }

    // Hashes: a whole run of letters and digits that is twelve or more hex characters.
    let mut start = None;
    for (index, byte) in bytes.iter().enumerate().chain(end) {
        match (byte.is_ascii_alphanumeric(), start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                let run = &text[from..index];
                let is_number = run.bytes().all(|byte| byte.is_ascii_digit());
                if run.len() >= 12 && !is_number && run.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    buffer.clear();
                    buffer.push_str(run);
                    buffer.make_ascii_lowercase();
                    found(TokenKind::Hash, &buffer);
                }
                start = None;
            }
            _ => {}
        }
    }

    // Hosts: what follows `//`, when it's two or more labels and the last is two or more letters.
    let host_character = |byte: &u8| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'.';
    for (index, _) in text.match_indices("//") {
        let from = index + 2;
        let length = bytes[from..]
            .iter()
            .take_while(|byte| host_character(byte))
            .count();
        let host = text[from..from + length].trim_matches(['.', '-']);
        let labels: Vec<_> = host.split('.').collect();
        let last = labels.last().copied().unwrap_or_default();
        if labels.len() >= 2
            && labels.iter().all(|label| !label.is_empty())
            && last.len() >= 2
            && last.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            buffer.clear();
            buffer.push_str(host);
            buffer.make_ascii_lowercase();
            found(TokenKind::Host, &buffer);
        }
    }

    // Words: runs of seven or more letters, in any script.
    let mut letters = 0;
    buffer.clear();
    for character in text.chars().chain([' ']) {
        if character.is_alphabetic() {
            letters += 1;
            buffer.extend(character.to_lowercase());
        } else {
            if letters >= 7 {
                found(TokenKind::Word, &buffer);
            }
            letters = 0;
            buffer.clear();
        }
    }
}

/// The tokens in the files being checked, each with the files it's in.
#[derive(Default)]
struct RepositoryTokens {
    files: usize,
    tokens: HashMap<String, (TokenKind, BTreeSet<String>)>,
    /// Files outside the notification files.
    other_files: usize,
    /// Words of `tokens` that the files outside the notification files use too.
    vocabulary: HashSet<String>,
}

impl RepositoryTokens {
    fn read(&mut self, path: &Path, name: String) {
        let bytes = fs::read(path).unwrap_or_else(|error| panic!("{name} can't be read: {error}"));
        self.files += 1;
        tokens(&String::from_utf8_lossy(&bytes), &mut |kind, token| {
            self.tokens
                .entry(token.to_string())
                .or_insert_with(|| (kind, BTreeSet::new()))
                .1
                .insert(name.clone());
        });
    }

    fn read_vocabulary(&mut self, path: &Path) {
        // A file that can't be read adds no words, which only makes the check stricter.
        let Ok(bytes) = fs::read(path) else {
            return;
        };
        self.other_files += 1;
        tokens(&String::from_utf8_lossy(&bytes), &mut |kind, token| {
            if kind == TokenKind::Word && self.tokens.contains_key(token) {
                self.vocabulary.insert(token.to_string());
            }
        });
    }
}

/// Where the recorded notes use a repository token.
#[derive(Default, Clone, Copy)]
struct Use {
    notes: u64,
    /// Whether the token is part of an object key or of a `type` value.
    in_api_names: bool,
}

#[derive(Default)]
struct SampleUses {
    lines: u64,
    notes: u64,
    uses: HashMap<String, Use>,
}

impl SampleUses {
    fn add(&mut self, other: SampleUses) {
        self.lines += other.lines;
        self.notes += other.notes;
        for (token, other) in other.uses {
            let found = self.uses.entry(token).or_default();
            found.notes += other.notes;
            found.in_api_names |= other.in_api_names;
        }
    }
}

fn nothing_checked(reason: &str) -> ! {
    panic!(
        "NOTHING WAS CHECKED: {reason} Set NOTIFICATIONS_CORPUS to the folders of recorded \
         notes and run this again; until it passes, notification work isn't ready to push."
    );
}

#[test]
#[ignore = "needs a folder of recorded notes; see the module comment"]
fn fixture_check() {
    let Some(folders) = std::env::var_os("NOTIFICATIONS_CORPUS") else {
        nothing_checked("NOTIFICATIONS_CORPUS isn't set, so there are no notes to compare with.");
    };
    let mut files = Vec::new();
    for folder in std::env::split_paths(&folders) {
        if !folder.is_dir() {
            nothing_checked(&format!("{} isn't a folder.", folder.display()));
        }
        files.extend(note_files(&folder));
    }
    if files.is_empty() {
        nothing_checked("the folders in NOTIFICATIONS_CORPUS hold no .jsonl files.");
    }

    let repository = repository_tokens();
    let sample = read_sample(&files, &repository);
    if sample.notes == 0 {
        nothing_checked("the .jsonl files in NOTIFICATIONS_CORPUS hold no notes.");
    }

    let mut by_kind: BTreeMap<TokenKind, u64> = BTreeMap::new();
    for (kind, _) in repository.tokens.values() {
        *by_kind.entry(*kind).or_default() += 1;
    }
    let mut shared = BTreeMap::new();
    let (mut common, mut api_names, mut vocabulary) = (BTreeMap::new(), 0, 0);
    for (token, found) in &sample.uses {
        let (kind, in_files) = &repository.tokens[token];
        let is_word = *kind == TokenKind::Word;
        if found.notes > STRUCTURE_NOTES {
            common.insert((*kind, token.as_str()), found.notes);
        } else if is_word && found.in_api_names {
            api_names += 1;
        } else if is_word && repository.vocabulary.contains(token) {
            vocabulary += 1;
        } else {
            shared.insert((*kind, token.as_str()), (found.notes, in_files));
        }
    }

    println!(
        "Files checked: {} (Git's files with `notification` or `styled_text` in their path, and \
         the files in NOTIFICATIONS_FIXTURE_CHECK_EXTRA)",
        repository.files
    );
    println!(
        "Tokens in them: {} ({})",
        repository.tokens.len(),
        by_kind
            .iter()
            .map(|(kind, count)| format!("{count} {}s", kind.name()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "Other repository files read for the words they use: {}",
        repository.other_files
    );
    println!(
        "Recorded notes read: {} from {} lines in {} files",
        sample.notes,
        sample.lines,
        files.len()
    );
    println!(
        "Tokens also in the notes: {}. Not counted as shared: {} in more than {STRUCTURE_NOTES} \
         notes, {api_names} words in an object key or a `type` value, {vocabulary} words the \
         rest of the repository uses",
        sample.uses.len(),
        common.len()
    );
    let common_values: Vec<_> = common
        .iter()
        .filter(|((kind, _), _)| *kind != TokenKind::Word)
        .map(|((kind, token), notes)| format!("{token} ({}, {notes} notes)", kind.name()))
        .collect();
    if !common_values.is_empty() {
        println!(
            "Numbers, hashes and hosts in more than {STRUCTURE_NOTES} notes: {}",
            common_values.join(", ")
        );
    }
    if shared.is_empty() {
        println!("PASSED: no file checked shares a value with the recorded notes.");
        return;
    }
    let mut message = format!(
        "FAILED: {} tokens in the files checked are also in recorded notes. Replace each with an \
         invented value:\n",
        shared.len()
    );
    for ((kind, token), (notes, in_files)) in &shared {
        let in_files: Vec<_> = in_files.iter().map(String::as_str).collect();
        message.push_str(&format!(
            "  {token} ({}, in {notes} notes)  in {}\n",
            kind.name(),
            in_files.join(", ")
        ));
    }
    panic!("{message}");
}

/// Reads every file Git would commit whose path names notifications or styled text, and every
/// file in the folders named in `NOTIFICATIONS_FIXTURE_CHECK_EXTRA`. Then reads Git's other files
/// for the words they use.
fn repository_tokens() -> RepositoryTokens {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("wp_api is inside the repository");
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| nothing_checked(&format!("git can't list the files: {error}.")));
    if !output.status.success() {
        nothing_checked("git can't list the repository's files.");
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    let (checked, others): (Vec<_>, Vec<_>) = listing
        .split('\0')
        .filter(|name| root.join(name).is_file())
        .partition(|name| {
            let name = name.to_lowercase();
            name.contains("notification") || name.contains("styled_text")
        });
    let mut repository = RepositoryTokens::default();
    for name in checked {
        repository.read(&root.join(name), name.to_string());
    }
    if repository.files == 0 {
        nothing_checked("no repository file has `notification` or `styled_text` in its path.");
    }
    if let Some(folders) = std::env::var_os("NOTIFICATIONS_FIXTURE_CHECK_EXTRA") {
        for folder in std::env::split_paths(&folders) {
            let mut extra: Vec<PathBuf> = fs::read_dir(&folder)
                .unwrap_or_else(|error| panic!("{} can't be read: {error}", folder.display()))
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.is_file())
                .collect();
            extra.sort();
            for path in extra {
                repository.read(&path, path.display().to_string());
            }
        }
    }
    for name in others {
        repository.read_vocabulary(&root.join(name));
    }
    repository
}

/// Reads the recorded notes on one thread per core, and counts where each repository token is
/// used.
fn read_sample(files: &[PathBuf], repository: &RepositoryTokens) -> SampleUses {
    let next = AtomicUsize::new(0);
    let total = Mutex::new(SampleUses::default());
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let mut uses = SampleUses::default();
                while let Some(file) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                    read_sample_file(file, repository, &mut uses);
                }
                total.lock().expect("no thread panicked").add(uses);
            });
        }
    });
    total.into_inner().expect("no thread panicked")
}

fn read_sample_file(file: &Path, repository: &RepositoryTokens, uses: &mut SampleUses) {
    let reader = BufReader::new(File::open(file).expect("the file can be opened"));
    let mut in_note = Vec::new();
    for line in reader.lines() {
        let line = line.expect("the file can be read");
        if line.trim().is_empty() {
            continue;
        }
        uses.lines += 1;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let note = match value.get("note") {
            Some(note) if note.is_object() => note,
            _ => &value,
        };
        uses.notes += 1;
        in_note.clear();
        walk(note, false, repository, uses, &mut in_note);
        in_note.sort_unstable();
        in_note.dedup();
        for token in &in_note {
            uses.uses.entry(token.clone()).or_default().notes += 1;
        }
    }
}

/// Collects the repository tokens in a note's values into `in_note`, and marks the words in its
/// keys and `type` values as names the API defines.
fn walk(
    value: &Value,
    is_type: bool,
    repository: &RepositoryTokens,
    uses: &mut SampleUses,
    in_note: &mut Vec<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                add(key, true, repository, uses, in_note);
                walk(value, key == "type", repository, uses, in_note);
            }
        }
        Value::Array(list) => {
            for value in list {
                walk(value, false, repository, uses, in_note);
            }
        }
        Value::String(text) => add(text, is_type, repository, uses, in_note),
        Value::Number(number) => add(&number.to_string(), false, repository, uses, in_note),
        Value::Bool(_) | Value::Null => {}
    }
}

/// Adds the repository tokens in `text` to `in_note`, or marks its words as names the API defines.
fn add(
    text: &str,
    is_name: bool,
    repository: &RepositoryTokens,
    uses: &mut SampleUses,
    in_note: &mut Vec<String>,
) {
    tokens(text, &mut |kind, token| {
        if !repository.tokens.contains_key(token) {
            return;
        }
        if is_name && kind == TokenKind::Word {
            uses.uses.entry(token.to_string()).or_default().in_api_names = true;
        } else {
            in_note.push(token.to_string());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_tokens(text: &str) -> Vec<(TokenKind, String)> {
        let mut found = Vec::new();
        tokens(text, &mut |kind, token| {
            found.push((kind, token.to_string()))
        });
        found.sort();
        found
    }

    #[test]
    fn finds_numbers_hashes_hosts_and_words() {
        assert_eq!(
            all_tokens("See https://Blog.Example.org/p/12345678?h=0A1B2C3D4E5F6A7B Wonderful."),
            vec![
                (TokenKind::Number, "12345678".to_string()),
                (TokenKind::Hash, "0a1b2c3d4e5f6a7b".to_string()),
                (TokenKind::Host, "blog.example.org".to_string()),
                (TokenKind::Word, "example".to_string()),
                (TokenKind::Word, "wonderful".to_string()),
            ]
        );
    }

    #[test]
    fn leaves_out_short_runs_file_names_and_version_numbers() {
        assert_eq!(
            all_tokens("id 12345, abcdef01234, v1.2.3, avatar.png, //x.y, words"),
            vec![]
        );
    }
}
