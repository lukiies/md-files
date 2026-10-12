//! Fact vocabularies and fact sets.
//!
//! The facts a directive may read are a closed set owned by the project that
//! writes the sources (for the contract templates: the BAU project). A
//! project declares its vocabulary in an `itamd-slownik.json` file placed in
//! the sources' folder or any folder above it; tools find it by walking up
//! from the document. A fact set (the facts of one configuration, used by a
//! compiler) is a JSON object of fact values.

use std::path::{Path, PathBuf};

use super::diag::Diagnostic;
use super::expr::{is_constant, Facts, Value, TIERS};
use crate::json::{self, Json};

/// The file name a project's vocabulary is declared under.
pub const VOCABULARY_FILE: &str = "itamd-slownik.json";

/// The type of a fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactType {
    /// `logiczny`: true or false.
    Logical,
    /// `poziom`: one of the tiers T1–T5 (the only ordered type).
    Tier,
    /// `wyliczeniowy`: one of the listed constants, or empty (not set).
    Enum(Vec<String>),
}

impl FactType {
    fn word(&self) -> &'static str {
        match self {
            FactType::Logical => "logiczny",
            FactType::Tier => "poziom",
            FactType::Enum(_) => "wyliczeniowy",
        }
    }
}

/// One fact of a vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactDef {
    pub name: String,
    pub kind: FactType,
    /// Short reader-facing name shown by viewers instead of the identifier.
    pub label: Option<String>,
    /// What the fact means and where its value comes from.
    pub description: Option<String>,
}

/// A project's fact vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    /// The standard version the vocabulary declares, e.g. `ITA-MD 1.0`.
    pub standard: String,
    /// Optional vocabulary name (`nazwa`).
    pub name: Option<String>,
    pub facts: Vec<FactDef>,
    /// The file it was loaded from, when loaded from disk.
    pub path: Option<PathBuf>,
}

/// Split `ITA-MD MAJOR.MINOR` into its numbers.
fn parse_version(s: &str) -> Option<(u32, u32)> {
    let rest = s.strip_prefix("ITA-MD ")?;
    let (major, minor) = rest.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Vocabulary {
    /// Parse a vocabulary file's text. Problems are reported as IMD-501
    /// (malformed) or IMD-502 (another major version of the standard).
    pub fn parse(text: &str) -> Result<Vocabulary, Vec<Diagnostic>> {
        let bad = |msg: String| vec![Diagnostic::new("IMD-501", 0, msg)];
        let root = json::parse(text).map_err(|e| bad(format!("niepoprawny JSON ({e})")))?;
        let Some(pairs) = root.as_object() else {
            return Err(bad("plik słownika musi zawierać obiekt JSON".into()));
        };
        let standard = match root.get("standard").and_then(Json::as_str) {
            Some(s) => s.to_string(),
            None => return Err(bad("brak pola „standard” (np. \"ITA-MD 1.0\")".into())),
        };
        let Some((major, minor)) = parse_version(&standard) else {
            return Err(bad(format!("pole „standard” ma postać \"ITA-MD MAJOR.MINOR\", jest \"{standard}\"")));
        };
        if major != super::MAJOR {
            return Err(vec![Diagnostic::new(
                "IMD-502",
                0,
                format!("słownik deklaruje {standard}; to narzędzie obsługuje {}", super::STANDARD),
            )]);
        }
        // Keys a newer MINOR may add are ignored; for this or an older
        // version an unknown key is a typo and is reported. (While MINOR is
        // 0 the comparison reduces to equality; it is written for 1.1+.)
        #[allow(clippy::absurd_extreme_comparisons)]
        let strict = minor <= super::MINOR;
        let mut errors = Vec::new();
        for (key, _) in pairs {
            if strict && !["standard", "nazwa", "opis", "fakty"].contains(&key.as_str()) {
                errors.push(format!("nieznany klucz „{key}”"));
            }
        }
        let name = root.get("nazwa").and_then(Json::as_str).map(str::to_string);
        let Some(fact_pairs) = root.get("fakty").and_then(Json::as_object) else {
            return Err(bad("brak obiektu „fakty”".into()));
        };
        let mut facts = Vec::new();
        for (fact, def) in fact_pairs {
            if !is_identifier(fact) {
                errors.push(format!(
                    "„{fact}” nie jest poprawnym identyfikatorem (litery łacińskie bez znaków diakrytycznych, cyfry, „_”)"
                ));
                continue;
            }
            if is_constant(fact) || ["and", "or", "not", "in", "True", "False"].contains(&fact.as_str()) {
                errors.push(format!("„{fact}” jest zastrzeżone (stała albo słowo wyrażenia)"));
                continue;
            }
            let Some(props) = def.as_object() else {
                errors.push(format!("definicja faktu „{fact}” musi być obiektem"));
                continue;
            };
            for (key, _) in props {
                if strict && !["typ", "etykieta", "opis", "wartości"].contains(&key.as_str()) {
                    errors.push(format!("fakt „{fact}”: nieznany klucz „{key}”"));
                }
            }
            let text_field = |key: &str, errors: &mut Vec<String>| match def.get(key) {
                None => None,
                Some(Json::String(s)) => Some(s.clone()),
                Some(other) => {
                    errors.push(format!("fakt „{fact}”: „{key}” musi być tekstem, jest {}", other.type_name()));
                    None
                }
            };
            let label = text_field("etykieta", &mut errors);
            let description = text_field("opis", &mut errors);
            let kind = match def.get("typ").and_then(Json::as_str) {
                Some("logiczny") => FactType::Logical,
                Some("poziom") => FactType::Tier,
                Some("wyliczeniowy") => {
                    let values: Option<Vec<String>> = match def.get("wartości") {
                        Some(Json::Array(items)) if !items.is_empty() => items
                            .iter()
                            .map(|v| v.as_str().filter(|s| is_constant(s)).map(str::to_string))
                            .collect(),
                        _ => None,
                    };
                    match values {
                        Some(v) => FactType::Enum(v),
                        None => {
                            errors.push(format!(
                                "fakt „{fact}”: typ „wyliczeniowy” wymaga niepustej listy „wartości” złożonej ze stałych ITA-MD"
                            ));
                            continue;
                        }
                    }
                }
                Some(other) => {
                    errors.push(format!(
                        "fakt „{fact}”: nieznany typ „{other}” (dozwolone: logiczny, poziom, wyliczeniowy)"
                    ));
                    continue;
                }
                None => {
                    errors.push(format!("fakt „{fact}”: brak pola „typ”"));
                    continue;
                }
            };
            if !matches!(kind, FactType::Enum(_)) && def.get("wartości").is_some() {
                errors.push(format!("fakt „{fact}”: „wartości” dotyczą tylko typu „wyliczeniowy”"));
            }
            facts.push(FactDef { name: fact.clone(), kind, label, description });
        }
        if !errors.is_empty() {
            return Err(errors.into_iter().map(|m| Diagnostic::new("IMD-501", 0, m)).collect());
        }
        Ok(Vocabulary { standard, name, facts, path: None })
    }

    /// Load a vocabulary file from disk.
    pub fn load(path: &Path) -> Result<Vocabulary, Vec<Diagnostic>> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            vec![Diagnostic::new("IMD-501", 0, format!("nie można odczytać {}: {e}", path.display()))]
        })?;
        let mut v = Vocabulary::parse(&text).map_err(|diags| {
            diags
                .into_iter()
                .map(|mut d| {
                    d.message = format!("{}: {}", path.display(), d.message);
                    d
                })
                .collect::<Vec<_>>()
        })?;
        v.path = Some(path.to_path_buf());
        Ok(v)
    }

    /// The vocabulary file that applies to documents in `dir`: the first
    /// `itamd-slownik.json` found in `dir` or a folder above it.
    pub fn discover(dir: &Path) -> Option<PathBuf> {
        dir.ancestors().map(|d| d.join(VOCABULARY_FILE)).find(|p| p.is_file())
    }

    /// The definition of fact `name`.
    pub fn get(&self, name: &str) -> Option<&FactDef> {
        self.facts.iter().find(|f| f.name == name)
    }

    /// The reader-facing label of fact `name`, if the vocabulary gives one.
    pub fn label(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|f| f.label.as_deref())
    }

    /// The vocabulary's name for messages: `nazwa`, else the file name.
    pub fn display_name(&self) -> String {
        if let Some(n) = &self.name {
            return n.clone();
        }
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| VOCABULARY_FILE.to_string())
    }

    /// Check a fact set against the vocabulary: every fact present (IMD-503),
    /// nothing extra (IMD-504), every value of its type (IMD-505).
    pub fn validate(&self, facts: &Facts) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for def in &self.facts {
            let Some(value) = facts.get(&def.name) else {
                out.push(Diagnostic::new("IMD-503", 0, format!("brak faktu „{}” w zestawie faktów", def.name)));
                continue;
            };
            let ok = match (&def.kind, value) {
                (FactType::Logical, Value::Bool(_)) => true,
                (FactType::Tier, Value::Text(s)) => TIERS.contains(&s.as_str()),
                (FactType::Enum(values), Value::Text(s)) => s.is_empty() || values.contains(s),
                _ => false,
            };
            if !ok {
                let shown = match value {
                    Value::Bool(b) => b.to_string(),
                    Value::Text(s) => format!("\"{s}\""),
                };
                let expected = match &def.kind {
                    FactType::Logical => "true albo false".to_string(),
                    FactType::Tier => TIERS.join(", "),
                    FactType::Enum(values) => format!("{} albo \"\"", values.join(", ")),
                };
                out.push(Diagnostic::new(
                    "IMD-505",
                    0,
                    format!(
                        "fakt „{}” (typ {}) ma wartość {shown}; oczekiwano: {expected}",
                        def.name,
                        def.kind.word()
                    ),
                ));
            }
        }
        for name in facts.keys() {
            if self.get(name).is_none() {
                out.push(Diagnostic::new(
                    "IMD-504",
                    0,
                    format!("fakt „{name}” nie należy do słownika „{}”", self.display_name()),
                ));
            }
        }
        out
    }
}

/// Read a fact set from JSON text: an object of fact values, or an object
/// whose `fakty` (or `facts`) member holds them — so a generator's manifest
/// can be used directly. `true`/`false` are logical facts, strings are text
/// facts, `null` is an empty text (not set). Other values are IMD-505;
/// unreadable JSON is IMD-506.
pub fn parse_facts(text: &str) -> Result<Facts, Vec<Diagnostic>> {
    let root = json::parse(text)
        .map_err(|e| vec![Diagnostic::new("IMD-506", 0, format!("niepoprawny JSON ({e})"))])?;
    let holder = match (root.get("fakty"), root.get("facts")) {
        (Some(f @ Json::Object(_)), _) | (None, Some(f @ Json::Object(_))) => f,
        _ => &root,
    };
    let Some(pairs) = holder.as_object() else {
        return Err(vec![Diagnostic::new("IMD-506", 0, "plik faktów musi zawierać obiekt JSON")]);
    };
    let mut facts = Facts::new();
    let mut errors = Vec::new();
    for (name, value) in pairs {
        let v = match value {
            Json::Bool(b) => Value::Bool(*b),
            Json::String(s) => Value::Text(s.clone()),
            Json::Null => Value::Text(String::new()),
            other => {
                errors.push(Diagnostic::new(
                    "IMD-505",
                    0,
                    format!("fakt „{name}” ma wartość typu {}; fakty są logiczne albo tekstowe", other.type_name()),
                ));
                continue;
            }
        };
        facts.insert(name.clone(), v);
    }
    if errors.is_empty() {
        Ok(facts)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "standard": "ITA-MD 1.0",
        "nazwa": "przykład",
        "fakty": {
            "tier": {"typ": "poziom", "etykieta": "Poziom", "opis": "Poziom usług"},
            "ef": {"typ": "logiczny", "etykieta": "EF/KSeF"},
            "reservation": {"typ": "wyliczeniowy", "wartości": ["L1", "L2"]}
        }
    }"#;

    fn codes(r: Result<Vocabulary, Vec<Diagnostic>>) -> Vec<&'static str> {
        r.err().unwrap().iter().map(|d| d.code).collect()
    }

    #[test]
    fn test_parse_good_vocabulary() {
        let v = Vocabulary::parse(GOOD).unwrap();
        assert_eq!(v.facts.len(), 3);
        assert_eq!(v.label("tier"), Some("Poziom"));
        assert_eq!(v.get("reservation").unwrap().kind, FactType::Enum(vec!["L1".into(), "L2".into()]));
        assert_eq!(v.display_name(), "przykład");
        assert!(v.get("nope").is_none());
    }

    #[test]
    fn test_vocabulary_errors() {
        assert_eq!(codes(Vocabulary::parse("{")), ["IMD-501"]);
        assert_eq!(codes(Vocabulary::parse("[]")), ["IMD-501"]);
        assert_eq!(codes(Vocabulary::parse(r#"{"fakty": {}}"#)), ["IMD-501"]);
        assert_eq!(codes(Vocabulary::parse(r#"{"standard": "1.0", "fakty": {}}"#)), ["IMD-501"]);
        assert_eq!(codes(Vocabulary::parse(r#"{"standard": "ITA-MD 2.0", "fakty": {}}"#)), ["IMD-502"]);
        let bad_facts = r#"{"standard": "ITA-MD 1.0", "fakty": {
            "poziom_usług": {"typ": "poziom"},
            "T4": {"typ": "logiczny"},
            "a": {"typ": "liczba"},
            "b": {"typ": "wyliczeniowy"},
            "c": {"typ": "wyliczeniowy", "wartości": ["zielony"]},
            "d": {},
            "e": {"typ": "logiczny", "etykieda": "x"},
            "f": {"typ": "logiczny", "wartości": ["L1"]},
            "g": "logiczny"
        }}"#;
        assert_eq!(codes(Vocabulary::parse(bad_facts)).len(), 9);
        // A newer MINOR may add keys; they are ignored.
        let newer = r#"{"standard": "ITA-MD 1.7", "stałe": [], "fakty": {"a": {"typ": "logiczny", "kolor": "x"}}}"#;
        assert!(Vocabulary::parse(newer).is_ok());
        let unknown_root = r#"{"standard": "ITA-MD 1.0", "stałe": [], "fakty": {}}"#;
        assert_eq!(codes(Vocabulary::parse(unknown_root)), ["IMD-501"]);
    }

    #[test]
    fn test_discover_walks_up() {
        let root = std::env::temp_dir().join("mdfiles_itamd_vocab_discover");
        let deep = root.join("a").join("b");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(root.join(VOCABULARY_FILE), GOOD).unwrap();
        assert_eq!(Vocabulary::discover(&deep), Some(root.join(VOCABULARY_FILE)));
        let loaded = Vocabulary::load(&root.join(VOCABULARY_FILE)).unwrap();
        assert_eq!(loaded.path, Some(root.join(VOCABULARY_FILE)));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_parse_facts_plain_and_manifest() {
        let f = parse_facts(r#"{"tier": "T3", "ef": true, "reservation": null}"#).unwrap();
        assert_eq!(f["tier"], Value::Text("T3".into()));
        assert_eq!(f["ef"], Value::Bool(true));
        assert_eq!(f["reservation"], Value::Text(String::new()));
        let m = parse_facts(r#"{"profile": "x", "facts": {"tier": "T4"}}"#).unwrap();
        assert_eq!(m.len(), 1);
        let p = parse_facts(r#"{"fakty": {"ef": false}}"#).unwrap();
        assert_eq!(p["ef"], Value::Bool(false));
        assert_eq!(parse_facts("nope").unwrap_err()[0].code, "IMD-506");
        assert_eq!(parse_facts("[1]").unwrap_err()[0].code, "IMD-506");
        assert_eq!(parse_facts(r#"{"n": 3}"#).unwrap_err()[0].code, "IMD-505");
    }

    #[test]
    fn test_validate_fact_set() {
        let v = Vocabulary::parse(GOOD).unwrap();
        let ok = parse_facts(r#"{"tier": "T2", "ef": false, "reservation": ""}"#).unwrap();
        assert!(v.validate(&ok).is_empty());
        let bad = parse_facts(r#"{"tier": "T7", "ef": "tak", "extra": true}"#).unwrap();
        let codes: Vec<_> = v.validate(&bad).iter().map(|d| d.code).collect();
        assert_eq!(codes, ["IMD-505", "IMD-505", "IMD-503", "IMD-504"]);
    }
}
