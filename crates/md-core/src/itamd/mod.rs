//! ITA-MD 1.0: conditional directives in Markdown sources.
//!
//! A source says in which configuration a block, a table row, or a fragment
//! of a sentence applies; a compiler evaluates the directives against the
//! facts of one configuration and produces the deliverable. The handbook
//! (`docs/ITA-MD/podrecznik-ITA-MD.md`) is the normative description; this
//! module implements it for the suite:
//!
//! * [`check`] — every diagnostic of the error catalogue for a source;
//! * [`annotate`] — the viewer's rendering (all branches visible, decorated);
//! * [`strip_directives`] — directives removed, every branch kept (what a
//!   converter that does not compile delivers, with a warning);
//! * [`parse_expression`], [`Vocabulary`], [`parse_facts`] — the expression
//!   language, fact vocabularies, and fact sets.
//!
//! Directives never apply inside code blocks, code spans, or HTML comments.
//!
//! ```text
//! dyrektywa = "⟦" słowo [ " " wyrażenie ] { " · " opcja } "⟧"
//! słowo     = "JEŻELI" | "ALBO JEŻELI" | "W PRZECIWNYM RAZIE" | "KONIEC" | "WIERSZ" | "TABELA"
//! opcja     = "zaślepka: " ( "numer" | "tytuł" | "brak" ) | "kolumny: poziom"
//! ```

mod analyze;
mod annotate;
mod compile;
mod diag;
mod expr;
mod lex;
mod strip;
mod vocab;

pub use analyze::Stats;
pub use annotate::{annotate, diagnostics_panel, dim_stubs, page_css, preview_toolbar, AnnotateOptions, Annotated, PAGE_JS};
pub use compile::{cleanup, compile, project, CompileOptions, Compiled, TIER_FACT};
pub use diag::{CatalogueEntry, Diagnostic, Severity, CATALOGUE};
pub use expr::{is_constant, parse as parse_expression, CmpOp, EvalError, Expr, ExprError, Facts, Value, TIERS};
pub use lex::{Keyword, Stub, DIRECTIVE_EMOJI};
pub use strip::strip_directives;
pub(crate) use strip::strip_counted;
pub use vocab::{parse_facts, FactDef, FactType, Vocabulary, VOCABULARY_FILE};

/// The standard version this implementation conforms to.
pub const STANDARD: &str = "ITA-MD 1.0";
/// Major version of the standard (a change needs migration of sources).
pub const MAJOR: u32 = 1;
/// Minor version of the standard (backward-compatible additions).
pub const MINOR: u32 = 0;
/// The stub sentence left by a block that does not apply.
pub const STUB_BLOCK: &str = "Nie dotyczy niniejszej Umowy.";
/// The stub left in a table cell by a row that does not apply.
pub const STUB_CELL: &str = "nie dotyczy niniejszej Umowy";

/// What a converter that does not compile says about a source with
/// directives (its output keeps every branch).
pub const WARNING_UNCOMPILED: &str =
    "plik zawiera dyrektywy ITA-MD 1.0 — to źródło, nie egzemplarz; użyj kompilatora";

/// The vocabulary that applies to documents in `dir` (see
/// [`Vocabulary::discover`]): `(None, [])` when there is none, `(None,
/// diagnostics)` when the file exists but cannot be used.
pub fn vocabulary_for(dir: &std::path::Path) -> (Option<Vocabulary>, Vec<Diagnostic>) {
    match Vocabulary::discover(dir) {
        None => (None, Vec::new()),
        Some(path) => match Vocabulary::load(&path) {
            Ok(v) => (Some(v), Vec::new()),
            Err(diags) => (None, diags),
        },
    }
}

/// Read a fact set from a JSON file (see [`parse_facts`]).
pub fn load_facts(path: &std::path::Path) -> Result<Facts, Vec<Diagnostic>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| vec![Diagnostic::new("IMD-506", 0, format!("nie można odczytać {}: {e}", path.display()))])?;
    parse_facts(&text).map_err(|diags| {
        diags
            .into_iter()
            .map(|mut d| {
                d.message = format!("{}: {}", path.display(), d.message);
                d
            })
            .collect()
    })
}

/// Sort diagnostics by line, then code (stable within equal keys).
pub(crate) fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| (a.line, a.code).cmp(&(b.line, b.code)));
}

/// Check a source: every lexical, structural, and expression problem, with
/// fact names checked against `vocabulary` when one is given. Sorted by line.
pub fn check(source: &str, vocabulary: Option<&Vocabulary>) -> Vec<Diagnostic> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let scan = lex::scan(source);
    let analysis = analyze::analyze(&scan, vocabulary);
    let mut out = scan.diagnostics;
    out.extend(analysis.diagnostics);
    sort(&mut out);
    out
}

/// How many directives a source holds (block and table lines, inline and
/// row directives; code and comments excluded).
pub fn count_directives(source: &str) -> usize {
    strip::strip_counted(source).1
}

/// True if a source holds at least one directive.
pub fn contains_directives(source: &str) -> bool {
    count_directives(source) > 0
}

#[cfg(test)]
mod handbook;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_sorts_by_line() {
        let d = check("a ⟦KONIEC⟧\n> 🔀 ⟦JEŻELI x⟧\n", None);
        let got: Vec<(usize, &str)> = d.iter().map(|d| (d.line, d.code)).collect();
        assert_eq!(got, [(1, "IMD-202"), (2, "IMD-101")]);
    }

    #[test]
    fn test_vocabulary_for_and_load_facts() {
        let root = std::env::temp_dir().join("mdfiles_itamd_vocabulary_for");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        assert!(matches!(vocabulary_for(&root.join("sub")), (None, d) if d.is_empty()));
        std::fs::write(root.join(VOCABULARY_FILE), "{ zepsuty").unwrap();
        let (v, d) = vocabulary_for(&root.join("sub"));
        assert!(v.is_none() && d[0].code == "IMD-501");
        std::fs::write(root.join(VOCABULARY_FILE), r#"{"standard": "ITA-MD 1.0", "fakty": {"ef": {"typ": "logiczny"}}}"#).unwrap();
        assert!(vocabulary_for(&root.join("sub")).0.is_some());
        std::fs::write(root.join("f.json"), r#"{"ef": true}"#).unwrap();
        assert_eq!(load_facts(&root.join("f.json")).unwrap()["ef"], Value::Bool(true));
        let missing = load_facts(&root.join("nope.json")).unwrap_err();
        assert_eq!(missing[0].code, "IMD-506");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_count_and_contains() {
        assert_eq!(count_directives("a⟦JEŻELI x⟧b⟦KONIEC⟧\n\n> 🔀 ⟦KONIEC⟧\n"), 3);
        assert!(!contains_directives("`⟦KONIEC⟧` and > 🔀 in text\n"));
        assert!(contains_directives("| ⟦WIERSZ a⟧x |\n"));
    }
}
