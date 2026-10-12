//! The handbook's examples are tests. Every fenced code block whose info
//! string carries an `itamd…` tag is checked against this implementation,
//! so the handbook (the single source of knowledge about the standard) and
//! the code cannot drift apart:
//!
//! * `markdown itamd id=X oczekuj=ok|IMD-nnn[,…] [slownik=V] [plik=P]` — a
//!   source; `check` must report exactly those codes (`ok`: nothing);
//! * `markdown itamd-bez-dyrektyw zrodlo=X` — `strip_directives(X)`;
//! * `markdown itamd-egzemplarz zrodlo=X fakty=F [slownik=V]` — `compile`;
//! * `text itamd-kompilacja zrodlo=X fakty=F [slownik=V]` — compilation
//!   fails with the codes that open the block's lines;
//! * `json itamd-slownik id=V [oczekuj=…] [plik=P]` — a vocabulary;
//! * `json itamd-fakty id=F [oczekuj=…] [plik=P]` — a fact set;
//! * `text itamd-wyrazenie oczekuj=ok|IMD-401` — one expression per line.
//!
//! `plik=P` ties an example to a file under `docs/ITA-MD/`: both must be
//! identical.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use super::*;

const HANDBOOK_FILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/ITA-MD/podrecznik-ITA-MD.md"));

/// The handbook with LF line endings: a Windows checkout may have CRLF, and
/// the examples are compared byte for byte with LF compiler output.
fn handbook() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| HANDBOOK_FILE.replace("\r\n", "\n"))
}
const CHANGELOG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/ITA-MD/CHANGELOG.md"));

fn docs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/ITA-MD")
}

struct Block {
    lang: String,
    tag: String,
    attrs: BTreeMap<String, String>,
    text: String,
    line: usize,
}

impl Block {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    fn need(&self, key: &str) -> &str {
        self.attr(key).unwrap_or_else(|| panic!("handbook line {}: `{}` needs `{key}=`", self.line, self.tag))
    }

    /// Expected codes: `None` for "ok", the set of codes otherwise.
    fn expected(&self) -> BTreeSet<String> {
        match self.attr("oczekuj") {
            None | Some("ok") => BTreeSet::new(),
            Some(list) => list.split(',').map(str::to_string).collect(),
        }
    }

    fn place(&self) -> String {
        format!("handbook line {} ({} {})", self.line, self.tag, self.attr("id").or(self.attr("zrodlo")).unwrap_or(""))
    }
}

fn blocks() -> Vec<Block> {
    let mut out = Vec::new();
    let mut current: Option<Block> = None;
    for (event, range) in Parser::new_ext(handbook(), crate::md_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let mut words = info.split_whitespace();
                let lang = words.next().unwrap_or("").to_string();
                let tag = words.next().unwrap_or("").to_string();
                let attrs = words
                    .filter_map(|w| w.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                let line = handbook()[..range.start].matches('\n').count() + 1;
                current = Some(Block { lang, tag, attrs, text: String::new(), line });
            }
            Event::Text(t) => {
                if let Some(b) = current.as_mut() {
                    b.text.push_str(&t);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(b) = current.take() {
                    if b.tag.starts_with("itamd") {
                        out.push(b);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn codes(diags: &[Diagnostic]) -> BTreeSet<String> {
    diags.iter().map(|d| d.code.to_string()).collect()
}

/// Named sources, vocabularies and fact sets of the handbook.
struct Registry {
    sources: BTreeMap<String, String>,
    vocabularies: BTreeMap<String, Vocabulary>,
    facts: BTreeMap<String, Facts>,
}

fn registry(all: &[Block]) -> Registry {
    let mut r = Registry { sources: BTreeMap::new(), vocabularies: BTreeMap::new(), facts: BTreeMap::new() };
    for b in all {
        let Some(id) = b.attr("id") else { continue };
        let fresh = match b.tag.as_str() {
            "itamd" => r.sources.insert(id.to_string(), b.text.clone()).is_none(),
            "itamd-slownik" => match Vocabulary::parse(&b.text) {
                Ok(v) => r.vocabularies.insert(id.to_string(), v).is_none(),
                Err(_) => true,
            },
            "itamd-fakty" => match parse_facts(&b.text) {
                Ok(f) => r.facts.insert(id.to_string(), f).is_none(),
                Err(_) => true,
            },
            _ => true,
        };
        assert!(fresh, "{}: duplicate id `{id}`", b.place());
    }
    r
}

#[test]
fn handbook_examples_hold() {
    let all = blocks();
    assert!(all.len() > 80, "found only {} tagged examples", all.len());
    let reg = registry(&all);
    let vocab_of = |b: &Block| b.attr("slownik").map(|v| reg.vocabularies.get(v).unwrap_or_else(|| panic!("{}: no vocabulary `{v}`", b.place())));
    let mut failures = Vec::new();
    for b in &all {
        if let Some(file) = b.attr("plik") {
            let on_disk = std::fs::read_to_string(docs_dir().join(file)).unwrap_or_else(|e| panic!("{}: {file}: {e}", b.place()));
            if on_disk.replace("\r\n", "\n") != b.text {
                failures.push(format!("{}: differs from {file}", b.place()));
            }
        }
        match b.tag.as_str() {
            "itamd" => {
                assert_eq!(b.lang, "markdown", "{}", b.place());
                let got = check(&b.text, vocab_of(b));
                if codes(&got) != b.expected() {
                    failures.push(format!("{}: expected {:?}, got {:#?}", b.place(), b.expected(), got));
                }
            }
            "itamd-bez-dyrektyw" => {
                let src = &reg.sources[b.need("zrodlo")];
                let got = strip_directives(src);
                if got != b.text {
                    failures.push(format!("{}: stripped text differs:\n--- got\n{got}--- handbook\n{}", b.place(), b.text));
                }
            }
            "itamd-egzemplarz" | "itamd-kompilacja" => {
                let src = &reg.sources[b.need("zrodlo")];
                let facts = reg.facts.get(b.need("fakty")).unwrap_or_else(|| panic!("{}: no fact set", b.place()));
                let result = compile(src, facts, &CompileOptions { vocabulary: vocab_of(b) });
                match (b.tag.as_str(), result) {
                    ("itamd-egzemplarz", Ok(c)) if c.text == b.text => {}
                    ("itamd-egzemplarz", Ok(c)) => failures.push(format!(
                        "{}: compiled text differs:\n--- got\n{}--- handbook\n{}",
                        b.place(),
                        c.text,
                        b.text
                    )),
                    ("itamd-egzemplarz", Err(e)) => failures.push(format!("{}: compilation failed: {e:#?}", b.place())),
                    (_, Ok(c)) => failures.push(format!("{}: expected a failure, compiled:\n{}", b.place(), c.text)),
                    (_, Err(e)) => {
                        let listed: BTreeSet<String> = b
                            .text
                            .lines()
                            .filter_map(|l| {
                                let l = l.split_once(": ").filter(|(p, _)| p.starts_with("linia ")).map(|(_, r)| r).unwrap_or(l);
                                l.split_whitespace().next().map(str::to_string)
                            })
                            .collect();
                        if codes(&e) != listed {
                            failures.push(format!("{}: expected {listed:?}, got {e:#?}", b.place()));
                        }
                        for line in b.text.lines() {
                            if !e.iter().any(|d| d.to_string() == line) {
                                failures.push(format!("{}: no diagnostic prints as {line:?}; got {e:#?}", b.place()));
                            }
                        }
                    }
                }
            }
            "itamd-slownik" => {
                let got = Vocabulary::parse(&b.text).err().map(|e| codes(&e)).unwrap_or_default();
                if got != b.expected() {
                    failures.push(format!("{}: expected {:?}, got {got:?}", b.place(), b.expected()));
                }
            }
            "itamd-fakty" => {
                let got = parse_facts(&b.text).err().map(|e| codes(&e)).unwrap_or_default();
                if got != b.expected() {
                    failures.push(format!("{}: expected {:?}, got {got:?}", b.place(), b.expected()));
                }
            }
            "itamd-wyrazenie" => {
                let ok = b.expected().is_empty();
                for line in b.text.lines().filter(|l| !l.trim().is_empty()) {
                    match (parse_expression(line), ok) {
                        (Ok(e), true) if e.canonical() == line => {}
                        (Ok(e), true) => failures.push(format!("{}: `{line}` is not canonical (`{}`)", b.place(), e.canonical())),
                        (Err(err), true) => failures.push(format!("{}: `{line}` should parse: {err}", b.place())),
                        (Ok(_), false) => failures.push(format!("{}: `{line}` should not parse", b.place())),
                        (Err(_), false) => {}
                    }
                }
            }
            other => failures.push(format!("handbook line {}: unknown example tag `{other}`", b.line)),
        }
    }
    assert!(failures.is_empty(), "{} handbook example(s) fail:\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn handbook_covers_the_error_catalogue() {
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for b in blocks() {
        covered.extend(b.expected());
        if b.tag == "itamd-kompilacja" {
            covered.extend(b.text.lines().filter_map(|l| l.split("IMD-").nth(1).map(|r| format!("IMD-{}", &r[..3]))));
        }
        if b.tag == "itamd-wyrazenie" && !b.expected().is_empty() {
            covered.insert("IMD-401".into());
        }
    }
    // IMD-603 is the compiler's self-check: no correct compiler produces it.
    let missing: Vec<&str> =
        CATALOGUE.iter().map(|e| e.code).filter(|c| *c != "IMD-603" && !covered.contains(*c)).collect();
    assert!(missing.is_empty(), "codes without a handbook example: {missing:?}");
}

#[test]
fn handbook_catalogue_table_matches_the_code() {
    for entry in CATALOGUE {
        let row = format!("| {} | {} | {} |", entry.code, entry.severity.label(), entry.title);
        assert!(handbook().contains(&row), "the handbook's error table lacks `{row}`");
    }
    let rows = handbook().lines().filter(|l| l.starts_with("| IMD-")).count();
    assert_eq!(rows, CATALOGUE.len(), "the handbook's error table has extra or missing rows");
}

#[test]
fn handbook_itself_has_no_directives_outside_code() {
    let diags = check(HANDBOOK_FILE, None);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(count_directives(HANDBOOK_FILE), 0);
}

#[test]
fn handbook_and_changelog_declare_this_version() {
    assert!(handbook().contains(&format!("| **Wersja standardu** | {STANDARD} |")));
    assert!(CHANGELOG.contains(&format!("## [{MAJOR}.{MINOR}]")));
}

#[test]
fn handbook_images_exist() {
    for (i, _) in handbook().match_indices("](obrazy/") {
        let rest = &handbook()[i + 2..];
        let path = &rest[..rest.find(')').unwrap()];
        assert!(docs_dir().join(path).is_file(), "missing image {path}");
    }
}
