//! Lexing: find the ITA-MD directives of a source and read each one.
//!
//! Every line is one of: code (never touched), a block directive line
//! (`<indent>> 🔀 ⟦…⟧`), a broken 🔀 line, or a text line that may carry
//! inline and row directives. Problems are reported with catalogue codes as
//! they are found; later passes (structure, expressions) add their own.

use std::ops::Range;

use super::diag::Diagnostic;
use super::expr::{self, Expr};
use crate::source::{self, Layout, SourceLine};

/// U+1F500, the emoji that turns a working note into a block directive.
pub const DIRECTIVE_EMOJI: char = '\u{1F500}';
const OPEN: char = '⟦';
const CLOSE: char = '⟧';

/// A directive keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    /// `JEŻELI`
    If,
    /// `ALBO JEŻELI`
    ElseIf,
    /// `W PRZECIWNYM RAZIE`
    Else,
    /// `KONIEC`
    End,
    /// `WIERSZ`
    Row,
    /// `TABELA`
    Table,
}

impl Keyword {
    const ALL: [Keyword; 6] =
        [Keyword::ElseIf, Keyword::Else, Keyword::If, Keyword::End, Keyword::Row, Keyword::Table];

    /// The keyword as written in a source.
    pub fn word(self) -> &'static str {
        match self {
            Keyword::If => "JEŻELI",
            Keyword::ElseIf => "ALBO JEŻELI",
            Keyword::Else => "W PRZECIWNYM RAZIE",
            Keyword::End => "KONIEC",
            Keyword::Row => "WIERSZ",
            Keyword::Table => "TABELA",
        }
    }

    fn takes_expression(self) -> bool {
        matches!(self, Keyword::If | Keyword::ElseIf | Keyword::Row)
    }
}

/// What a compiler leaves when a block or row does not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stub {
    /// `numer`: the unit's designation and the stub sentence.
    Number,
    /// `tytuł`: the designation with its title and the stub sentence.
    Title,
    /// `brak`: nothing.
    None,
}

impl Stub {
    /// The option value as written in a source.
    pub fn word(self) -> &'static str {
        match self {
            Stub::Number => "numer",
            Stub::Title => "tytuł",
            Stub::None => "brak",
        }
    }
}

/// One directive as read from the source.
#[derive(Debug, Clone)]
pub struct Directive {
    pub keyword: Keyword,
    /// The expression as written (trimmed); empty when absent.
    pub expr_text: String,
    /// The parsed expression, when present and well-formed.
    pub expr: Option<Expr>,
    /// `zaślepka: …`, when given.
    pub stub: Option<Stub>,
    /// The directive exactly as written, brackets included.
    pub raw: String,
    /// 1-based source line.
    pub line: usize,
    /// Byte range of the directive within its line.
    pub span: Range<usize>,
}

/// A bracketed piece of a text line.
#[derive(Debug, Clone)]
pub(crate) enum Token {
    Directive(Directive),
    /// Something that is not a directive: a stray bracket or an unknown
    /// keyword. Kept so viewers can mark it.
    Bad { span: Range<usize> },
}

impl Token {
    pub fn span(&self) -> &Range<usize> {
        match self {
            Token::Directive(d) => &d.span,
            Token::Bad { span, .. } => span,
        }
    }
}

/// What a source line is.
#[derive(Debug, Clone)]
pub(crate) enum LineKind {
    /// Inside a code block or a block-level HTML comment.
    Code,
    /// A block directive line. `recovered` lines broke a lexical rule of the
    /// line shape (reported as IMD-004) but hold a readable directive.
    Block { indent: usize, directive: Directive, recovered: bool },
    /// A 🔀 line without a usable directive (reported).
    BadBlock,
    /// Any other line, with its bracketed pieces in order.
    Text { tokens: Vec<Token> },
}

/// A lexed source.
pub(crate) struct Scan<'a> {
    pub text: &'a str,
    pub lines: Vec<SourceLine<'a>>,
    pub kinds: Vec<LineKind>,
    pub layout: Layout,
    pub diagnostics: Vec<Diagnostic>,
}

impl Scan<'_> {
    /// 1-based line number of byte `pos`.
    pub fn line_of(&self, pos: usize) -> usize {
        self.lines.partition_point(|l| l.start <= pos).max(1)
    }
}

/// Lex `text` (which must not start with a BOM).
pub(crate) fn scan(text: &str) -> Scan<'_> {
    let layout = Layout::of(text);
    let lines = source::lines(text);
    let mut diagnostics = Vec::new();
    let mut kinds = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        kinds.push(classify(line, idx + 1, &layout, &mut diagnostics));
    }
    Scan { text, lines, kinds, layout, diagnostics }
}

fn classify(line: &SourceLine, no: usize, layout: &Layout, diags: &mut Vec<Diagnostic>) -> LineKind {
    let t = line.text;
    let lead = t.len() - t.trim_start().len();
    let body = &t[lead..];
    if !body.is_empty() && layout.is_block_excluded(line.start + lead) {
        return LineKind::Code;
    }
    if let Some(after) = body.strip_prefix('>') {
        if after.trim_start().starts_with(DIRECTIVE_EMOJI) {
            return block_line(t, lead, after, no, diags);
        }
    }
    LineKind::Text { tokens: text_tokens(line, no, layout, diags) }
}

/// Read a line whose blockquote content starts with 🔀.
fn block_line(t: &str, lead: usize, after: &str, no: usize, diags: &mut Vec<Diagnostic>) -> LineKind {
    let mut shape: Vec<&str> = Vec::new();
    if t[..lead].chars().any(|c| c != ' ') {
        shape.push("wcięcie linii dyrektywy składa się wyłącznie ze spacji");
    }
    if !after.starts_with(' ') || after[1..].starts_with(char::is_whitespace) {
        shape.push("po „>” stoi dokładnie jedna spacja");
    }
    let mut rest = &after.trim_start()[DIRECTIVE_EMOJI.len_utf8()..];
    if let Some(r) = rest.strip_prefix('\u{FE0F}') {
        shape.push("po 🔀 nie może stać selektor wariantu U+FE0F (emoji bez dodatków)");
        rest = r;
    }
    let Some(open) = rest.find(OPEN) else {
        diags.push(Diagnostic::new(
            "IMD-004",
            no,
            "linia „> 🔀” bez dyrektywy; emoji 🔀 jest zastrzeżone dla dyrektyw ITA-MD",
        ));
        return LineKind::BadBlock;
    };
    if !rest[..open].trim().is_empty() {
        diags.push(Diagnostic::new("IMD-004", no, "między 🔀 a „⟦” nie może być tekstu"));
        return LineKind::BadBlock;
    }
    if &rest[..open] != " " {
        shape.push("między 🔀 a „⟦” stoi dokładnie jedna spacja");
    }
    let Some(close_rel) = rest[open..].find(CLOSE) else {
        diags.push(Diagnostic::new("IMD-001", no, "dyrektywa blokowa nie ma nawiasu zamykającego „⟧”"));
        return LineKind::BadBlock;
    };
    let close = open + close_rel + CLOSE.len_utf8();
    let tail = &rest[close..];
    // Byte offset of `rest` within the line.
    let base = t.len() - rest.len();
    let span = base + open..base + close;
    let directive = read(&rest[open..close], no, span, true, diags);
    if !tail.trim().is_empty() {
        let what = if tail.contains(OPEN) {
            "w linii blokowej stoi dokładnie jedna dyrektywa"
        } else {
            "po dyrektywie w linii blokowej nie może być tekstu"
        };
        diags.push(Diagnostic::new("IMD-004", no, what));
        return LineKind::BadBlock;
    }
    let Some(directive) = directive else {
        return LineKind::BadBlock;
    };
    if directive.keyword == Keyword::Row {
        diags.push(Diagnostic::new(
            "IMD-306",
            no,
            "WIERSZ stoi na początku pierwszej komórki wiersza tabeli, nie w osobnej linii (linia cytatu przerwałaby tabelę)",
        ));
        return LineKind::BadBlock;
    }
    for why in &shape {
        diags.push(Diagnostic::new("IMD-004", no, *why));
    }
    LineKind::Block { indent: lead, directive, recovered: !shape.is_empty() }
}

/// Find the bracketed pieces of an ordinary line.
fn text_tokens(line: &SourceLine, no: usize, layout: &Layout, diags: &mut Vec<Diagnostic>) -> Vec<Token> {
    let t = line.text;
    let mut tokens = Vec::new();
    let mut open: Option<usize> = None;
    for (i, c) in t.char_indices() {
        if (c != OPEN && c != CLOSE) || layout.is_excluded(line.start + i) {
            continue;
        }
        if c == OPEN {
            if let Some(o) = open {
                diags.push(Diagnostic::new("IMD-001", no, format!("„⟦” w kolumnie {} nie ma pary „⟧” przed następnym „⟦”", col(t, o))));
                tokens.push(Token::Bad { span: o..i });
            }
            open = Some(i);
        } else {
            let end = i + CLOSE.len_utf8();
            match open.take() {
                Some(o) => match read(&t[o..end], no, o..end, false, diags) {
                    Some(d) => tokens.push(Token::Directive(d)),
                    None => tokens.push(Token::Bad { span: o..end }),
                },
                None => {
                    diags.push(Diagnostic::new("IMD-003", no, format!("„⟧” w kolumnie {} bez „⟦”", col(t, i))));
                    tokens.push(Token::Bad { span: i..end });
                }
            }
        }
    }
    if let Some(o) = open {
        diags.push(Diagnostic::new("IMD-001", no, format!("„⟦” w kolumnie {} nie ma pary „⟧” w tej linii", col(t, o))));
        tokens.push(Token::Bad { span: o..t.len() });
    }
    tokens
}

/// 1-based character column of byte `i` in `t`.
fn col(t: &str, i: usize) -> usize {
    t[..i].chars().count() + 1
}

/// Hint for a word that is almost a keyword.
fn keyword_hint(inner: &str) -> &'static str {
    let first: String = inner.chars().take_while(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    match first.as_str() {
        "JEŻELI" | "JEZELI" | "JEŚLI" | "JESLI" | "IF" => " (czy chodziło o „JEŻELI”? słowa kluczowe pisze się wielkimi literami, z polskimi znakami)",
        "ALBO" | "ELIF" => " (czy chodziło o „ALBO JEŻELI”?)",
        "W" | "INACZEJ" | "ELSE" => " (czy chodziło o „W PRZECIWNYM RAZIE”?)",
        "KONIEC" | "END" => " (czy chodziło o „KONIEC”? słowa kluczowe pisze się wielkimi literami)",
        "WIERSZ" | "ROW" => " (czy chodziło o „WIERSZ”?)",
        "TABELA" | "TABLE" => " (czy chodziło o „TABELA”?)",
        _ => "",
    }
}

/// Read one directive (`raw` includes both brackets). Returns `None` when
/// the bracket does not start with a keyword (IMD-002).
pub(crate) fn read(raw: &str, no: usize, span: Range<usize>, block: bool, diags: &mut Vec<Diagnostic>) -> Option<Directive> {
    let inner = &raw[OPEN.len_utf8()..raw.len() - CLOSE.len_utf8()];
    let keyword = Keyword::ALL.into_iter().find(|k| {
        inner
            .strip_prefix(k.word())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    });
    let Some(keyword) = keyword else {
        let shown: String = inner.chars().take(24).collect();
        diags.push(Diagnostic::new(
            "IMD-002",
            no,
            format!("„⟦{shown}…”: po „⟦” musi stać słowo kluczowe (JEŻELI, ALBO JEŻELI, W PRZECIWNYM RAZIE, KONIEC, WIERSZ, TABELA){}", keyword_hint(inner)),
        ));
        return None;
    };
    let before = diags.len();
    let rest = &inner[keyword.word().len()..];
    let arg = rest.trim();
    let mut parts = arg.split('·');
    let expr_text = parts.next().unwrap_or("").trim().to_string();
    let mut stub = None;
    let mut tier_columns = false;
    let mut canonical_opts: Vec<String> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for part in parts {
        let p = part.trim();
        let Some((name, value)) = p.split_once(':') else {
            let msg = if p.is_empty() {
                "pusta opcja (podwójny albo końcowy „·”)".to_string()
            } else {
                format!("„{p}”: opcja ma postać „nazwa: wartość”")
            };
            diags.push(Diagnostic::new("IMD-006", no, msg));
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        let known = match name {
            "zaślepka" => Some("zaślepka"),
            "kolumny" => Some("kolumny"),
            _ => None,
        };
        let Some(name) = known else {
            let hint = match name {
                "zaslepka" | "Zaślepka" | "stub" => " (czy chodziło o „zaślepka”?)",
                "kolumna" | "Kolumny" | "columns" => " (czy chodziło o „kolumny”?)",
                _ => "",
            };
            diags.push(Diagnostic::new("IMD-006", no, format!("nieznana opcja „{name}”{hint}; opcje ITA-MD 1.0: zaślepka, kolumny")));
            continue;
        };
        if seen.contains(&name) {
            diags.push(Diagnostic::new("IMD-009", no, format!("opcja „{name}” podana więcej niż raz")));
            continue;
        }
        seen.push(name);
        let allowed = match name {
            "zaślepka" => (keyword == Keyword::If && block) || keyword == Keyword::Row,
            _ => keyword == Keyword::Table,
        };
        if !allowed {
            let where_ = match name {
                "zaślepka" => "„zaślepka” przysługuje tylko blokowemu JEŻELI i dyrektywie WIERSZ",
                _ => "„kolumny” przysługuje tylko dyrektywie TABELA",
            };
            diags.push(Diagnostic::new("IMD-008", no, format!("{where_}; tu stoi przy {}{}", keyword.word(), if block { " (blok)" } else { " (w linii)" })));
            continue;
        }
        match name {
            "zaślepka" => {
                stub = match value {
                    "numer" => Some(Stub::Number),
                    "tytuł" => Some(Stub::Title),
                    "brak" => Some(Stub::None),
                    _ => {
                        let hint = if value == "tytul" { " (czy chodziło o „tytuł”?)" } else { "" };
                        diags.push(Diagnostic::new("IMD-007", no, format!("„zaślepka: {value}”{hint}; dozwolone: numer, tytuł, brak")));
                        continue;
                    }
                };
            }
            _ => {
                if value != "poziom" {
                    diags.push(Diagnostic::new("IMD-007", no, format!("„kolumny: {value}”; dozwolone: poziom")));
                    continue;
                }
                tier_columns = true;
            }
        }
        canonical_opts.push(format!("{name}: {value}"));
    }
    let mut expr = None;
    if keyword.takes_expression() {
        if expr_text.is_empty() {
            diags.push(Diagnostic::new("IMD-010", no, format!("{} wymaga wyrażenia, np. „⟦{} tier >= T4⟧”", keyword.word(), keyword.word())));
        } else {
            match expr::parse(&expr_text) {
                Ok(e) => expr = Some(e),
                Err(e) => diags.push(Diagnostic::new("IMD-401", no, format!("„{expr_text}”: {}", e))),
            }
        }
    } else if !expr_text.is_empty() {
        diags.push(Diagnostic::new("IMD-011", no, format!("{} nie przyjmuje wyrażenia (jest „{expr_text}”)", keyword.word())));
    }
    if keyword == Keyword::Table && !tier_columns && !seen.contains(&"kolumny") {
        diags.push(Diagnostic::new("IMD-305", no, "TABELA wymaga opcji „kolumny: poziom”: ⟦TABELA · kolumny: poziom⟧"));
    }
    let ok = diags.len() == before;
    if ok {
        let mut canon = String::from(OPEN);
        canon.push_str(keyword.word());
        if let Some(e) = &expr {
            canon.push(' ');
            canon.push_str(&e.canonical());
        }
        for o in &canonical_opts {
            canon.push_str(" · ");
            canon.push_str(o);
        }
        canon.push(CLOSE);
        if canon != raw {
            diags.push(Diagnostic::new("IMD-005", no, format!("„{raw}”: zapis kanoniczny to „{canon}”")));
        }
    }
    Some(Directive {
        keyword,
        expr_text,
        expr,
        stub,
        raw: raw.to_string(),
        line: no,
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(diags: &[Diagnostic]) -> Vec<&'static str> {
        diags.iter().map(|d| d.code).collect()
    }

    fn read_one(raw: &str, block: bool) -> (Option<Directive>, Vec<&'static str>) {
        let mut diags = Vec::new();
        let d = read(raw, 1, 0..raw.len(), block, &mut diags);
        (d, codes(&diags))
    }

    #[test]
    fn test_read_canonical_directives() {
        for (raw, block) in [
            ("⟦JEŻELI tier >= T4⟧", true),
            ("⟦JEŻELI tier >= T4 · zaślepka: numer⟧", true),
            ("⟦JEŻELI existing · zaślepka: tytuł⟧", true),
            ("⟦ALBO JEŻELI holidays⟧", false),
            ("⟦W PRZECIWNYM RAZIE⟧", false),
            ("⟦KONIEC⟧", true),
            ("⟦WIERSZ standby24 · zaślepka: numer⟧", false),
            ("⟦TABELA · kolumny: poziom⟧", true),
        ] {
            let (d, c) = read_one(raw, block);
            assert!(c.is_empty(), "{raw}: {c:?}");
            assert!(d.is_some());
        }
        let (d, _) = read_one("⟦JEŻELI tier >= T4 · zaślepka: brak⟧", true);
        let d = d.unwrap();
        assert_eq!((d.keyword, d.stub, d.expr_text.as_str()), (Keyword::If, Some(Stub::None), "tier >= T4"));
    }

    #[test]
    fn test_read_reports_each_problem() {
        let cases: &[(&str, bool, &[&str])] = &[
            ("⟦JEZELI x⟧", false, &["IMD-002"]),
            ("⟦jeżeli x⟧", false, &["IMD-002"]),
            ("⟦JEŻELIx⟧", false, &["IMD-002"]),
            ("⟦JEŻELI  tier >= T4⟧", true, &["IMD-005"]),
            ("⟦JEŻELI tier>=T4⟧", true, &["IMD-005"]),
            ("⟦JEŻELI x·zaślepka:numer⟧", true, &["IMD-005"]),
            ("⟦JEŻELI x ⟧", true, &["IMD-005"]),
            ("⟦JEŻELI x · kolor: czerwony⟧", true, &["IMD-006"]),
            ("⟦JEŻELI x · zaslepka: numer⟧", true, &["IMD-006"]),
            ("⟦JEŻELI x · · zaślepka: numer⟧", true, &["IMD-006"]),
            ("⟦JEŻELI x · zaślepka numer⟧", true, &["IMD-006"]),
            ("⟦JEŻELI x · zaślepka: nic⟧", true, &["IMD-007"]),
            ("⟦TABELA · kolumny: wszystkie⟧", true, &["IMD-007"]),
            ("⟦JEŻELI x · zaślepka: numer⟧", false, &["IMD-008"]),
            ("⟦ALBO JEŻELI x · zaślepka: numer⟧", true, &["IMD-008"]),
            ("⟦JEŻELI x · kolumny: poziom⟧", true, &["IMD-008"]),
            ("⟦JEŻELI x · zaślepka: numer · zaślepka: brak⟧", true, &["IMD-009"]),
            ("⟦JEŻELI⟧", true, &["IMD-010"]),
            ("⟦WIERSZ · zaślepka: numer⟧", false, &["IMD-010"]),
            ("⟦KONIEC x⟧", true, &["IMD-011"]),
            ("⟦W PRZECIWNYM RAZIE tier == T1⟧", false, &["IMD-011"]),
            ("⟦TABELA⟧", true, &["IMD-305"]),
            ("⟦JEŻELI tier = T4⟧", true, &["IMD-401"]),
        ];
        for (raw, block, expected) in cases {
            let (_, c) = read_one(raw, *block);
            assert_eq!(&c, expected, "{raw}");
        }
    }

    fn kinds(text: &str) -> (Vec<LineKind>, Vec<&'static str>) {
        let s = scan(text);
        let c = codes(&s.diagnostics);
        (s.kinds, c)
    }

    #[test]
    fn test_block_lines_and_their_shape() {
        let (k, c) = kinds("> 🔀 ⟦JEŻELI tier >= T4⟧\n\n   > 🔀 ⟦KONIEC⟧\n");
        assert!(c.is_empty(), "{c:?}");
        assert!(matches!(&k[0], LineKind::Block { indent: 0, recovered: false, .. }));
        assert!(matches!(&k[2], LineKind::Block { indent: 3, .. }));
        for bad in [
            ">  🔀 ⟦KONIEC⟧",
            ">🔀 ⟦KONIEC⟧",
            "> 🔀⟦KONIEC⟧",
            "> 🔀  ⟦KONIEC⟧",
            "> 🔀\u{FE0F} ⟦KONIEC⟧",
        ] {
            let (k, c) = kinds(bad);
            assert_eq!(c, ["IMD-004"], "{bad:?}");
            assert!(matches!(&k[0], LineKind::Block { recovered: true, .. }), "{bad:?}");
        }
        for (bad, code) in [
            ("> 🔀 a note", "IMD-004"),
            ("> 🔀 tekst ⟦KONIEC⟧", "IMD-004"),
            ("> 🔀 ⟦KONIEC⟧ tekst", "IMD-004"),
            ("> 🔀 ⟦KONIEC⟧⟦KONIEC⟧", "IMD-004"),
            ("> 🔀 ⟦KONIEC", "IMD-001"),
            ("> 🔀 ⟦KONIECZNIE⟧", "IMD-002"),
            ("> 🔀 ⟦WIERSZ x⟧", "IMD-306"),
        ] {
            let (k, c) = kinds(bad);
            assert_eq!(c, [code], "{bad:?}");
            assert!(matches!(&k[0], LineKind::BadBlock), "{bad:?}");
        }
    }

    #[test]
    fn test_tab_indentation() {
        // At the top level a tab makes an indented code block: not a directive.
        let (k, c) = kinds("\t> 🔀 ⟦KONIEC⟧\n");
        assert!(c.is_empty() && matches!(k[0], LineKind::Code));
        // Inside a list item it is a blockquote, and the tab breaks the shape.
        let (k, c) = kinds("1. item\n\n\t> 🔀 ⟦KONIEC⟧\n");
        assert_eq!(c, ["IMD-004"]);
        assert!(matches!(&k[2], LineKind::Block { recovered: true, .. }));
    }

    #[test]
    fn test_text_tokens_and_stray_brackets() {
        let (k, c) = kinds("a⟦JEŻELI x⟧b⟦KONIEC⟧c\n");
        assert!(c.is_empty());
        let LineKind::Text { tokens } = &k[0] else { panic!() };
        assert_eq!(tokens.len(), 2);
        assert_eq!(kinds("a ⟦JEŻELI x b\n").1, ["IMD-001"]);
        assert_eq!(kinds("a ⟦JEŻELI x ⟦KONIEC⟧\n").1, ["IMD-001"]);
        assert_eq!(kinds("a ⟧ b\n").1, ["IMD-003"]);
        assert_eq!(kinds("a ⟦IF x⟧ b\n").1, ["IMD-002"]);
    }

    #[test]
    fn test_code_is_never_lexed() {
        let text = "`⟦JEŻELI x⟧` and ``⟧``\n\n```md\n> 🔀 ⟦JEŻELI⟧\n⟦broken\n```\n\n    > 🔀 ⟦KONIEC⟧\n\n<!-- ⟦KONIEC⟧ -->\n";
        let (k, c) = kinds(text);
        assert!(c.is_empty(), "{c:?}");
        let LineKind::Text { tokens } = &k[0] else { panic!() };
        assert!(tokens.is_empty());
        assert!(matches!(k[3], LineKind::Code));
        assert!(matches!(k[7], LineKind::Code));
        assert!(matches!(k[9], LineKind::Code));
    }

    #[test]
    fn test_line_of() {
        let s = scan("a\nb\nc");
        assert_eq!(s.line_of(0), 1);
        assert_eq!(s.line_of(2), 2);
        assert_eq!(s.line_of(4), 3);
    }
}
