//! The viewer's rendering of a source. Directives become HTML frames and
//! chips; the text around them stays Markdown, so the result renders through
//! the normal pipeline. Every branch stays visible — a viewer never
//! evaluates — and the page gets a toolbar to hide the decorations.
//!
//! * a block branch is a framed region with a coloured bar and a header chip
//!   ("JEŻELI Poziom ≥ T4"), `W PRZECIWNYM RAZIE` in another tint, the stub
//!   kind as a badge;
//! * inline directives are small chips around a highlighted span;
//! * a row directive is a chip in the first cell of a tinted row;
//! * a table directive is a caption chip above its table;
//! * anything broken is shown as a red chip with the raw text, and every
//!   diagnostic is listed in a panel at the top.

use std::collections::BTreeSet;

use super::analyze::{self, BlockRole, Stats, TokenRole};
use super::diag::{Diagnostic, Severity};
use super::lex::{self, Directive, Keyword, LineKind, Stub, Token};
use super::vocab::Vocabulary;
use super::{STUB_BLOCK, STUB_CELL};

/// What [`annotate`] needs besides the source.
#[derive(Default)]
pub struct AnnotateOptions<'a> {
    /// The project's fact vocabulary: labels for facts, unknown-name checks.
    pub vocabulary: Option<&'a Vocabulary>,
    /// Problems found outside the source (e.g. an unreadable vocabulary
    /// file), listed together with the source's own.
    pub extra: Vec<Diagnostic>,
    /// Leave out the toolbar and the diagnostics panel (a caller that
    /// composes its own page header, like a failed compile preview).
    pub bare: bool,
}

/// An annotated source.
pub struct Annotated {
    /// Markdown with HTML frames and chips, ready for rendering.
    pub markdown: String,
    pub stats: Stats,
    /// Everything found, sorted by line.
    pub diagnostics: Vec<Diagnostic>,
}

/// Annotate `source` for a viewer. A source without directives (and
/// without directive errors) is returned unchanged.
pub fn annotate(source: &str, opts: &AnnotateOptions) -> Annotated {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let scan = lex::scan(source);
    let analysis = analyze::analyze(&scan, opts.vocabulary);
    let mut diagnostics: Vec<Diagnostic> = scan.diagnostics.iter().chain(&analysis.diagnostics).cloned().collect();
    if !analysis.stats.any() && diagnostics.is_empty() {
        return Annotated { markdown: source.to_string(), stats: analysis.stats, diagnostics };
    }
    diagnostics.extend(opts.extra.iter().cloned());
    super::sort(&mut diagnostics);

    let label =|name: &str| opts.vocabulary.and_then(|v| v.label(name)).map(str::to_string);
    let chips = Chips { vocab: opts.vocabulary, label: &label, diagnostics: &diagnostics };

    let mut out = String::with_capacity(source.len() * 2);
    if !opts.bare {
        out.push_str(&toolbar(&analysis.stats, &diagnostics, opts.vocabulary));
        out.push_str("\n\n");
        if !diagnostics.is_empty() {
            out.push_str(&diagnostics_panel(&diagnostics));
            out.push_str("\n\n");
        }
    }

    let n = scan.lines.len();
    for idx in 0..n {
        let line = &scan.lines[idx];
        let nl = line.newline();
        match &scan.kinds[idx] {
            LineKind::BadBlock => {
                // A 🔀 line that holds no usable directive: shown in place.
                let lead = line.text.len() - line.text.trim_start().len();
                out.push_str(nl);
                out.push_str(&line.text[..lead]);
                out.push_str(&format!("<div class=\"imd-blad-linia\">{}</div>", chips.broken(line.text.trim(), idx + 1)));
                out.push_str(nl);
                if idx + 1 < n && !scan.lines[idx + 1].is_blank() {
                    out.push_str(nl);
                }
                continue;
            }
            LineKind::Block { indent, directive, recovered } => {
                let html = match analysis.block_roles[idx] {
                    Some(BlockRole::Open(_)) => {
                        format!("<div class=\"imd-blok\">{}", chips.branch(directive, *recovered))
                    }
                    Some(BlockRole::Branch(_)) => format!("</div>{}", chips.branch(directive, *recovered)),
                    Some(BlockRole::Close(_)) => "</div></div>".to_string(),
                    Some(BlockRole::Table) => chips.table(directive),
                    _ => format!("<div class=\"imd-blad-linia\">{}</div>", chips.broken(&directive.raw, directive.line)),
                };
                out.push_str(nl);
                out.push_str(&" ".repeat(*indent));
                out.push_str(&html);
                out.push_str(nl);
                if idx + 1 < n && !scan.lines[idx + 1].is_blank() {
                    out.push_str(nl);
                }
                continue;
            }
            LineKind::Text { tokens } if !tokens.is_empty() => {
                let roles = &analysis.token_roles[idx];
                let text = line.text;
                let mut pos = 0;
                for (tok, role) in tokens.iter().zip(roles) {
                    let span = tok.span();
                    out.push_str(&text[pos..span.start]);
                    pos = span.end;
                    out.push_str(&match (tok, role) {
                        (Token::Directive(d), TokenRole::Open) => format!(
                            "<span class=\"imd-w\">{}<span class=\"imd-t imd-t-jezeli\">",
                            chips.inline(d)
                        ),
                        (Token::Directive(d), TokenRole::Branch) => format!(
                            "</span>{}<span class=\"imd-t {}\">",
                            chips.inline(d),
                            if d.keyword == Keyword::Else { "imd-t-inaczej" } else { "imd-t-albo" }
                        ),
                        (Token::Directive(d), TokenRole::Close) => format!("</span>{}</span>", chips.inline(d)),
                        (Token::Directive(d), TokenRole::Row) => chips.row(d),
                        _ => chips.broken(&text[span.clone()], idx + 1),
                    });
                }
                out.push_str(&text[pos..]);
            }
            _ => out.push_str(line.text),
        }
        out.push_str(line.ending);
    }
    Annotated { markdown: out, stats: analysis.stats, diagnostics }
}

/// Builds chip HTML.
struct Chips<'a> {
    vocab: Option<&'a Vocabulary>,
    label: &'a dyn Fn(&str) -> Option<String>,
    diagnostics: &'a [Diagnostic],
}

impl Chips<'_> {
    fn human(&self, d: &Directive) -> String {
        match &d.expr {
            Some(e) => e.humanize(self.label),
            None => d.expr_text.clone(),
        }
    }

    /// Tooltip: the directive as written, its line, and what its facts mean.
    fn title(&self, d: &Directive) -> String {
        let mut t = format!("{} — linia {}", d.raw, d.line);
        if let (Some(e), Some(v)) = (&d.expr, self.vocab) {
            let mut seen = BTreeSet::new();
            for name in e.facts() {
                if !seen.insert(name) {
                    continue;
                }
                if let Some(desc) = v.get(name).and_then(|f| f.description.as_deref()) {
                    t.push_str(&format!("\n{name}: {desc}"));
                }
            }
        }
        t
    }

    fn keyword_class(k: Keyword) -> &'static str {
        match k {
            Keyword::If => "imd-k-jezeli",
            Keyword::ElseIf => "imd-k-albo",
            Keyword::Else => "imd-k-inaczej",
            Keyword::End => "imd-k-koniec",
            Keyword::Row => "imd-k-wiersz",
            Keyword::Table => "imd-k-tabela",
        }
    }

    fn chip(&self, d: &Directive, small: bool) -> String {
        let class = Self::keyword_class(d.keyword);
        let size = if small { " imd-maly" } else { "" };
        let body = match d.keyword {
            Keyword::End => "⟧".to_string(),
            Keyword::Else => "<b>W PRZECIWNYM RAZIE</b>".to_string(),
            k => format!("<b>{}</b> {}", k.word(), esc(&self.human(d))),
        };
        format!("<span class=\"imd-chip {class}{size}\" title=\"{}\">{body}</span>", esc(&self.title(d)))
    }

    fn badge(stub: Stub, default: bool, row: bool) -> String {
        let what = match (stub, row) {
            (Stub::Number, false) => "egzemplarz zachowa oznaczenie jednostki ze zdaniem „Nie dotyczy niniejszej Umowy.”",
            (Stub::Title, false) => "egzemplarz zachowa oznaczenie i tytuł jednostki ze zdaniem „Nie dotyczy niniejszej Umowy.”",
            (Stub::None, false) => "fragment zniknie z egzemplarza bez śladu",
            (Stub::Number, true) => "wiersz zachowa pierwszą komórkę, a w kolejnej będzie „nie dotyczy niniejszej Umowy”",
            (Stub::Title, true) => "wiersz zachowa dwie pierwsze komórki, a w kolejnej będzie „nie dotyczy niniejszej Umowy”",
            (Stub::None, true) => "wiersz zniknie z egzemplarza",
        };
        let (class, note) = if default { (" imd-domyslna", " (domyślna)") } else { ("", "") };
        format!(
            "<span class=\"imd-badge{class}\" title=\"{}\">zaślepka: {}{note}</span>",
            esc(&format!("Gdy warunek nie jest spełniony, {what}.")),
            stub.word()
        )
    }

    /// The header of a block branch (opens the branch's frame). A recovered
    /// line (wrong spacing around `>`/🔀, reported as IMD-004) still frames
    /// its branch, with its chip outlined as faulty.
    fn branch(&self, d: &Directive, recovered: bool) -> String {
        let tint = match d.keyword {
            Keyword::ElseIf => "imd-g-albo",
            Keyword::Else => "imd-g-inaczej",
            _ => "imd-g-jezeli",
        };
        let badge = if d.keyword == Keyword::If {
            Self::badge(d.stub.unwrap_or(Stub::Number), d.stub.is_none(), false)
        } else {
            String::new()
        };
        let mut chip = self.chip(d, false);
        if recovered {
            chip = chip.replacen("class=\"imd-chip ", "class=\"imd-chip imd-niekanon ", 1);
        }
        format!(
            "<div class=\"imd-gal {tint}\" data-imd-linia=\"{}\"><div class=\"imd-glowa\">{chip}{badge}</div>",
            d.line
        )
    }

    fn inline(&self, d: &Directive) -> String {
        self.chip(d, true)
    }

    fn row(&self, d: &Directive) -> String {
        let badge = d.stub.map(|s| Self::badge(s, false, true)).unwrap_or_default();
        format!("{}{badge}", self.chip(d, true))
    }

    fn table(&self, d: &Directive) -> String {
        format!(
            "<div class=\"imd-tabela\"><span class=\"imd-chip imd-k-tabela\" title=\"{}\"><b>TABELA</b> kolumny: poziom</span><span class=\"imd-opis\">egzemplarz zachowa pierwszą kolumnę, kolumny bez nazwy Poziomu i kolumnę Poziomu Umowy</span></div>",
            esc(&self.title(d))
        )
    }

    /// Raw text of something broken, marked with the line's error codes.
    fn broken(&self, raw: &str, line: usize) -> String {
        let why: Vec<String> = self
            .diagnostics
            .iter()
            .filter(|d| d.line == line && d.severity == Severity::Error)
            .map(|d| format!("{}: {}", d.code, d.message))
            .collect();
        format!("<span class=\"imd-chip imd-zly\" title=\"{}\">{}</span>", esc(&why.join("\n")), esc(raw))
    }
}

/// The sticky toolbar: standard, the show/hide switch, counts, problems.
fn toolbar(stats: &Stats, diagnostics: &[Diagnostic], vocab: Option<&Vocabulary>) -> String {
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    let warnings = diagnostics.len() - errors;
    let mut counts = Vec::new();
    for (name, n) in [("bloki", stats.blocks), ("w linii", stats.inline), ("wiersze", stats.rows), ("tabele", stats.tables)] {
        if n > 0 {
            counts.push(format!("{name}: {n}"));
        }
    }
    let mut out = String::from(
        "<div class=\"imd-pasek\" id=\"imd-pasek\" role=\"toolbar\" aria-label=\"Dyrektywy ITA-MD\">\
         <span class=\"imd-std\" title=\"Dokument zawiera dyrektywy warunkowe ITA-MD 1.0: to źródło, nie egzemplarz\">ITA-MD 1.0</span>\
         <span class=\"imd-lbl\">Warunki:</span>\
         <span class=\"imd-przelacznik\" role=\"group\" aria-label=\"Warunki\">\
         <button type=\"button\" data-imd=\"pokaz\" aria-pressed=\"true\">pokaż</button>\
         <button type=\"button\" data-imd=\"ukryj\" aria-pressed=\"false\">ukryj</button></span>",
    );
    if !counts.is_empty() {
        out.push_str(&format!("<span class=\"imd-licznik\">{}</span>", esc(&counts.join(" · "))));
    }
    if let Some(v) = vocab {
        let path = v.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        out.push_str(&format!(
            "<span class=\"imd-slownik\" title=\"{}\">słownik: {}</span>",
            esc(&path),
            esc(&v.display_name())
        ));
    }
    if errors > 0 {
        out.push_str(&format!("<a class=\"imd-bledy\" href=\"#imd-diag\">błędy: {errors}</a>"));
    }
    if warnings > 0 {
        out.push_str(&format!("<a class=\"imd-ostrzezenia\" href=\"#imd-diag\">ostrzeżenia: {warnings}</a>"));
    }
    out.push_str(PROFILE_BUTTON);
    out.push_str("</div>");
    out
}

/// Opens a fact set for a compiled preview; shown only inside mdread.
const PROFILE_BUTTON: &str = "<button type=\"button\" class=\"imd-akcja imd-tylko-czytnik\" data-imd-akcja=\"profil\" title=\"Wczytaj plik faktów (JSON) i pokaż egzemplarz skompilowany dla tej konfiguracji\">Podgląd dla profilu…</button>";

/// The toolbar of a compiled preview (mdread's "Podgląd dla profilu…"):
/// which fact set is shown, and the way back to the source. `result` is
/// `Ok(stubs)` for a successful compilation, `Err(errors)` when it failed.
pub fn preview_toolbar(facts_name: &str, result: Result<usize, usize>) -> String {
    let state = match result {
        Ok(stubs) => format!("<span class=\"imd-licznik\">zaślepki: {stubs}</span>"),
        Err(errors) => format!("<a class=\"imd-bledy\" href=\"#imd-diag\">podgląd niemożliwy — błędy: {errors}</a>"),
    };
    format!(
        "<div class=\"imd-pasek imd-podglad\" id=\"imd-pasek\" role=\"toolbar\" aria-label=\"Podgląd egzemplarza ITA-MD\">\
         <span class=\"imd-std\">ITA-MD 1.0</span><span class=\"imd-lbl\">Podgląd egzemplarza dla faktów:</span>\
         <span class=\"imd-slownik\">{}</span>{state}\
         <button type=\"button\" class=\"imd-akcja\" data-imd-akcja=\"zrodlo\">Pokaż źródło</button>\
         <button type=\"button\" class=\"imd-akcja\" data-imd-akcja=\"profil\">Zmień profil…</button></div>",
        esc(facts_name)
    )
}

/// Dim the two stub wordings in rendered HTML (a compiled deliverable's
/// "not applicable" units read as secondary text).
pub fn dim_stubs(html: &str) -> String {
    html.replace(STUB_BLOCK, &format!("<span class=\"imd-nd\">{STUB_BLOCK}</span>"))
        .replace(STUB_CELL, &format!("<span class=\"imd-nd\">{STUB_CELL}</span>"))
}

/// The list of diagnostics shown above a document (an HTML block).
pub fn diagnostics_panel(diagnostics: &[Diagnostic]) -> String {
    const MAX: usize = 100;
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    let warnings = diagnostics.len() - errors;
    let class = if errors > 0 { "imd-diag" } else { "imd-diag imd-diag-ostrz" };
    let verdict = if errors > 0 { " — źródło nie jest zgodne ze standardem" } else { "" };
    let mut out = format!(
        "<div class=\"{class}\" id=\"imd-diag\" role=\"alert\"><div class=\"imd-diag-tytul\">ITA-MD 1.0 — błędy: {errors}, ostrzeżenia: {warnings}{verdict}</div><ul>"
    );
    for d in diagnostics.iter().take(MAX) {
        let place = if d.line > 0 { format!(" linia {}", d.line) } else { String::new() };
        out.push_str(&format!(
            "<li class=\"imd-d-{}\"><span class=\"imd-kod\">{}</span>{} — {}</li>",
            if d.is_error() { "blad" } else { "ostrz" },
            d.code,
            place,
            esc(&d.message)
        ));
    }
    if diagnostics.len() > MAX {
        out.push_str(&format!("<li>… i {} więcej</li>", diagnostics.len() - MAX));
    }
    out.push_str("</ul></div>");
    out
}

/// Escape text for HTML that sits inside Markdown: HTML specials plus every
/// character Markdown could read as syntax (including `|`, which would split
/// a table cell).
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            '|' => o.push_str("&#124;"),
            '*' => o.push_str("&#42;"),
            '_' => o.push_str("&#95;"),
            '`' => o.push_str("&#96;"),
            '[' => o.push_str("&#91;"),
            ']' => o.push_str("&#93;"),
            '\\' => o.push_str("&#92;"),
            '~' => o.push_str("&#126;"),
            '\n' => o.push_str("&#10;"),
            c => o.push(c),
        }
    }
    o
}

/// Stylesheet for annotated pages (light or dark palette). `plain_line` is
/// the page's ordinary table border colour, restored when decorations are
/// hidden.
pub fn page_css(dark: bool, plain_line: &str) -> String {
    let palette = if dark {
        "--imd-if:#4493f8;--imd-if-bg:rgba(56,139,253,0.07);--imd-if-chip:#0c2d6b;--imd-if-ink:#cae8ff;--imd-if-mark:rgba(68,147,248,0.18);\
         --imd-elif:#ab7df8;--imd-elif-bg:rgba(171,125,248,0.07);--imd-elif-chip:#2e1a55;--imd-elif-ink:#e8dbff;--imd-elif-mark:rgba(171,125,248,0.18);\
         --imd-else:#d29922;--imd-else-bg:rgba(210,153,34,0.07);--imd-else-chip:#3d2a00;--imd-else-ink:#f8e3a1;--imd-else-mark:rgba(210,153,34,0.18);\
         --imd-row:#3fb950;--imd-row-bg:rgba(46,160,67,0.10);--imd-row-chip:#0f3d1f;--imd-row-ink:#aff5b4;\
         --imd-tab:#39c5cf;--imd-tab-chip:#0b3a3e;--imd-tab-ink:#b3f0f3;\
         --imd-badge:#30363d;--imd-badge-ink:#e6edf3;--imd-badge-line:#6e7681;\
         --imd-err:#f85149;--imd-err-bg:#3c1618;--imd-err-ink:#ffdcd7;--imd-warn-bg:#3d2a00;--imd-warn-ink:#f8e3a1;\
         --imd-bar-bg:rgba(30,30,30,0.94);--imd-bar-line:#444;--imd-btn:#2d2d2d;--imd-btn-on:#4493f8;--imd-btn-on-ink:#0d1117;--imd-nd:#9aa0a6;"
    } else {
        "--imd-if:#0969da;--imd-if-bg:#f5f9ff;--imd-if-chip:#ddf4ff;--imd-if-ink:#0a3069;--imd-if-mark:rgba(9,105,218,0.11);\
         --imd-elif:#8250df;--imd-elif-bg:#faf7ff;--imd-elif-chip:#f1e8ff;--imd-elif-ink:#3e1f79;--imd-elif-mark:rgba(130,80,223,0.12);\
         --imd-else:#bf8700;--imd-else-bg:#fffbeb;--imd-else-chip:#fff1c2;--imd-else-ink:#5c3c00;--imd-else-mark:rgba(191,135,0,0.14);\
         --imd-row:#1a7f37;--imd-row-bg:#f0faf3;--imd-row-chip:#dafbe1;--imd-row-ink:#0f4a20;\
         --imd-tab:#0e7c86;--imd-tab-chip:#d2f4f2;--imd-tab-ink:#07484e;\
         --imd-badge:#eef1f4;--imd-badge-ink:#3a424b;--imd-badge-line:#8c959f;\
         --imd-err:#cf222e;--imd-err-bg:#ffebe9;--imd-err-ink:#82071e;--imd-warn-bg:#fff8c5;--imd-warn-ink:#5c3c00;\
         --imd-bar-bg:rgba(255,255,255,0.94);--imd-bar-line:#d0d7de;--imd-btn:#f6f8fa;--imd-btn-on:#0969da;--imd-btn-on-ink:#ffffff;--imd-nd:#5f6368;"
    };
    format!(
        r#":root {{ {palette} }}
.imd-pasek {{ position: sticky; top: 0; z-index: 20; display: flex; flex-wrap: wrap; align-items: center; gap: 6px 12px; margin: -20px -20px 16px; padding: 8px 20px; background: var(--imd-bar-bg); border-bottom: 1px solid var(--imd-bar-line); font: 12px/1.5 -apple-system, "Segoe UI", system-ui, sans-serif; }}
.imd-std {{ font-weight: 700; letter-spacing: .02em; }}
.imd-przelacznik {{ display: inline-flex; }}
.imd-przelacznik button {{ font: inherit; color: inherit; background: var(--imd-btn); border: 1px solid var(--imd-bar-line); padding: 1px 12px; cursor: pointer; }}
.imd-przelacznik button:first-child {{ border-radius: 999px 0 0 999px; }}
.imd-przelacznik button:last-child {{ border-radius: 0 999px 999px 0; border-left: 0; }}
.imd-przelacznik button[aria-pressed="true"] {{ background: var(--imd-btn-on); border-color: var(--imd-btn-on); color: var(--imd-btn-on-ink); font-weight: 600; }}
.imd-przelacznik button:focus-visible, .imd-bledy:focus-visible, .imd-ostrzezenia:focus-visible {{ outline: 2px solid var(--imd-if); outline-offset: 2px; }}
.imd-slownik {{ border: 1px dashed var(--imd-badge-line); border-radius: 4px; padding: 0 6px; }}
.imd-akcja {{ font: inherit; color: inherit; background: var(--imd-btn); border: 1px solid var(--imd-bar-line); border-radius: 999px; padding: 1px 12px; cursor: pointer; }}
.imd-akcja:focus-visible {{ outline: 2px solid var(--imd-if); outline-offset: 2px; }}
.imd-tylko-czytnik {{ display: none; }}
.imd-czytnik .imd-tylko-czytnik {{ display: inline-block; }}
.imd-podglad {{ border-bottom: 2px solid var(--imd-if); }}
.imd-nd {{ color: var(--imd-nd); font-style: italic; }}
.imd-bledy, .imd-ostrzezenia {{ border-radius: 999px; padding: 0 8px; font-weight: 600; text-decoration: none; }}
.imd-bledy {{ color: var(--imd-err-ink); background: var(--imd-err-bg); border: 1px solid var(--imd-err); }}
.imd-ostrzezenia {{ color: var(--imd-warn-ink); background: var(--imd-warn-bg); border: 1px solid var(--imd-else); }}
.imd-diag {{ border: 1px solid var(--imd-err); background: var(--imd-err-bg); color: var(--imd-err-ink); border-radius: 6px; padding: 8px 12px; margin: 0 0 16px; font: 13px/1.5 -apple-system, "Segoe UI", system-ui, sans-serif; }}
.imd-diag-ostrz {{ border-color: var(--imd-else); background: var(--imd-warn-bg); color: var(--imd-warn-ink); }}
.imd-diag-tytul {{ font-weight: 700; }}
.imd-diag ul {{ margin: 4px 0 0; padding-left: 18px; }}
.imd-kod {{ font-family: "Cascadia Code", Consolas, monospace; font-weight: 700; }}
.imd-blok {{ margin: 10px 0; }}
.imd-gal {{ border-left: 4px solid var(--imd-if); background: var(--imd-if-bg); border-radius: 0 6px 6px 0; padding: 1px 12px; }}
.imd-gal + .imd-gal {{ margin-top: 3px; }}
.imd-g-albo {{ border-left-color: var(--imd-elif); background: var(--imd-elif-bg); }}
.imd-g-inaczej {{ border-left-color: var(--imd-else); background: var(--imd-else-bg); }}
.imd-glowa {{ display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 7px 0 0; }}
.imd-gal > :last-child {{ margin-bottom: 8px; }}
.imd-chip {{ display: inline-block; font: 600 11.5px/1.55 -apple-system, "Segoe UI", system-ui, sans-serif; padding: 0 8px; border-radius: 999px; border: 1px solid var(--imd-if); background: var(--imd-if-chip); color: var(--imd-if-ink); text-indent: 0; }}
.imd-chip b {{ font-weight: 800; letter-spacing: .03em; }}
.imd-maly {{ display: inline; font-size: 10.5px; padding: 0 6px; margin: 0 2px; border-radius: 9px; vertical-align: 1px; -webkit-box-decoration-break: clone; box-decoration-break: clone; }}
.imd-k-albo {{ border-color: var(--imd-elif); background: var(--imd-elif-chip); color: var(--imd-elif-ink); }}
.imd-k-inaczej {{ border-color: var(--imd-else); background: var(--imd-else-chip); color: var(--imd-else-ink); }}
.imd-k-koniec {{ border-color: transparent; background: none; color: var(--imd-if); font-size: 13px; font-weight: 800; padding: 0 1px; margin: 0 1px; }}
.imd-k-wiersz {{ border-color: var(--imd-row); background: var(--imd-row-chip); color: var(--imd-row-ink); margin-left: 0; }}
.imd-k-tabela {{ border-color: var(--imd-tab); background: var(--imd-tab-chip); color: var(--imd-tab-ink); }}
.imd-zly {{ border-color: var(--imd-err); background: var(--imd-err-bg); color: var(--imd-err-ink); font-family: "Cascadia Code", Consolas, monospace; font-weight: 500; }}
.imd-niekanon {{ outline: 2px dashed var(--imd-err); outline-offset: 1px; }}
.imd-badge {{ display: inline-block; font: 500 11px/1.55 -apple-system, "Segoe UI", system-ui, sans-serif; padding: 0 7px; margin: 0 2px; border-radius: 4px; border: 1px dashed var(--imd-badge-line); background: var(--imd-badge); color: var(--imd-badge-ink); vertical-align: 1px; }}
.imd-domyslna {{ font-style: italic; }}
.imd-t {{ border-radius: 3px; padding: 0 1px; -webkit-box-decoration-break: clone; box-decoration-break: clone; }}
.imd-t-jezeli {{ background: var(--imd-if-mark); }}
.imd-t-albo {{ background: var(--imd-elif-mark); }}
.imd-t-inaczej {{ background: var(--imd-else-mark); }}
tr:has(.imd-k-wiersz) > td {{ background: var(--imd-row-bg); }}
.imd-tabela {{ display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin: 12px 0 6px; font: 12px/1.5 -apple-system, "Segoe UI", system-ui, sans-serif; }}
.imd-tabela + table th {{ border-top: 3px solid var(--imd-tab); }}
.imd-blad-linia {{ margin: 8px 0; }}
.imd-ukryj .imd-glowa, .imd-ukryj .imd-chip, .imd-ukryj .imd-badge, .imd-ukryj .imd-tabela {{ display: none !important; }}
.imd-ukryj .imd-blok {{ margin: 0; }}
.imd-ukryj .imd-gal {{ border: 0; background: none; padding: 0; margin: 0; border-radius: 0; }}
.imd-ukryj .imd-t {{ background: none; padding: 0; }}
.imd-ukryj tr:has(.imd-k-wiersz) > td {{ background: none; }}
.imd-ukryj .imd-tabela + table th {{ border-top: 1px solid {plain_line}; }}
@media print {{ .imd-pasek {{ display: none; }} }}
"#
    )
}

/// The show/hide switch. Inside mdread the choice is sent to the viewer
/// (which remembers it); in a browser it is kept in local storage.
pub const PAGE_JS: &str = r#"
(function(){
  var root = document.documentElement;
  function apply(hidden){
    root.classList.toggle('imd-ukryj', hidden);
    var bs = document.querySelectorAll('#imd-pasek [data-imd]');
    for (var i = 0; i < bs.length; i++) {
      bs[i].setAttribute('aria-pressed', String((bs[i].getAttribute('data-imd') === 'ukryj') === hidden));
    }
  }
  function save(hidden){
    try { if (window.ipc && window.ipc.postMessage) { window.ipc.postMessage(hidden ? 'itamd-ukryj' : 'itamd-pokaz'); return; } } catch (e) {}
    try { localStorage.setItem('itamd-warunki', hidden ? 'ukryj' : 'pokaz'); } catch (e) {}
  }
  if (!window.ipc) {
    try { var v = localStorage.getItem('itamd-warunki'); if (v) root.classList.toggle('imd-ukryj', v === 'ukryj'); } catch (e) {}
  }
  if (window.ipc) root.classList.add('imd-czytnik');
  document.addEventListener('click', function(e){
    var a = e.target && e.target.closest ? e.target.closest('#imd-pasek [data-imd-akcja]') : null;
    if (a) {
      try { window.ipc.postMessage('itamd-' + a.getAttribute('data-imd-akcja')); } catch (err) {}
      return;
    }
    var b = e.target && e.target.closest ? e.target.closest('#imd-pasek [data-imd]') : null;
    if (!b) return;
    var hidden = b.getAttribute('data-imd') === 'ukryj';
    apply(hidden);
    save(hidden);
  });
  function init(){ apply(root.classList.contains('imd-ukryj')); }
  if (document.readyState !== 'loading') init(); else document.addEventListener('DOMContentLoaded', init);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown_to_html;

    fn ann(src: &str) -> Annotated {
        annotate(src, &AnnotateOptions::default())
    }

    #[test]
    fn test_plain_source_is_unchanged() {
        let src = "# Tytuł\n\n> 📝 **[WORKING NOTE]** x\n\ntekst\n";
        let a = ann(src);
        assert_eq!(a.markdown, src);
        assert!(!a.stats.any());
        // Directives inside code do not count either.
        let code = "```md\n> 🔀 ⟦JEŻELI x⟧\n```\n";
        assert_eq!(ann(code).markdown, code);
    }

    #[test]
    fn test_block_becomes_frames_that_survive_rendering() {
        let src = "Przed.\n\n> 🔀 ⟦JEŻELI tier >= T4 · zaślepka: tytuł⟧\n\n6. **Punkt.** treść\n> 🔀 ⟦W PRZECIWNYM RAZIE⟧\n\n6. Inny.\n> 🔀 ⟦KONIEC⟧\n\nPo.\n";
        let a = ann(src);
        assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        assert_eq!(a.stats.blocks, 1);
        let html = markdown_to_html(&a.markdown, None);
        assert!(html.contains("<div class=\"imd-blok\"><div class=\"imd-gal imd-g-jezeli\""), "{html}");
        assert!(html.contains("<b>JEŻELI</b> tier ≥ T4"), "{html}");
        assert!(html.contains("zaślepka: tytuł"), "{html}");
        assert!(html.contains("imd-g-inaczej"), "{html}");
        assert!(html.contains("<b>W PRZECIWNYM RAZIE</b>"), "{html}");
        assert!(html.contains("<strong>Punkt.</strong>"), "markdown inside frames still renders: {html}");
        assert!(html.contains("<ol start=\"6\">"), "{html}");
        assert!(html.contains("</div></div>"), "{html}");
        // Directives as written survive only in tooltips.
        assert_eq!(html.matches('⟦').count(), html.matches("title=\"⟦").count(), "{html}");
        assert!(html.contains("id=\"imd-pasek\""));
    }

    #[test]
    fn test_default_stub_badge_is_marked() {
        let a = ann("> 🔀 ⟦JEŻELI ef⟧\n\n6. x\n> 🔀 ⟦KONIEC⟧\n");
        assert!(a.markdown.contains("zaślepka: numer (domyślna)"));
    }

    #[test]
    fn test_inline_groups_become_chips_and_spans() {
        let src = "Czas ⟦JEŻELI tier >= T4⟧określają **Załączniki A i D**⟦W PRZECIWNYM RAZIE⟧określa Załącznik A⟦KONIEC⟧.\n";
        let a = ann(src);
        let html = markdown_to_html(&a.markdown, None);
        assert!(html.contains("<span class=\"imd-w\">"), "{html}");
        assert!(html.contains("imd-t imd-t-jezeli\">określają <strong>Załączniki A i D</strong></span>"), "{html}");
        assert!(html.contains("imd-t imd-t-inaczej\">określa Załącznik A</span>"), "{html}");
        assert!(html.contains("imd-k-koniec"), "{html}");
        assert_eq!(a.stats.inline, 1);
    }

    #[test]
    fn test_rows_and_tables_stay_tables() {
        let src = "> 🔀 ⟦TABELA · kolumny: poziom⟧\n\n| Poziom | T1 | T2 |\n|---|---|---|\n| ⟦WIERSZ tier == T1 · zaślepka: numer⟧**T1** | a | b |\n";
        let a = ann(src);
        assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        let html = markdown_to_html(&a.markdown, None);
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("class=\"imd-tabela\""), "{html}");
        assert!(html.contains("<td><span class=\"imd-chip imd-k-wiersz imd-maly\""), "{html}");
        assert!(html.contains("<strong>T1</strong></td>"), "{html}");
        assert!(html.contains("zaślepka: numer"), "{html}");
        assert_eq!((a.stats.rows, a.stats.tables), (1, 1));
    }

    #[test]
    fn test_errors_are_listed_and_marked() {
        let a = ann("> 🔀 ⟦JEŻELI a⟧\n\ntekst ⟦JEZELI b⟧ i ⟦KONIEC⟧.\n");
        let codes: Vec<_> = a.diagnostics.iter().map(|d| d.code).collect();
        assert!(codes.contains(&"IMD-101") && codes.contains(&"IMD-002") && codes.contains(&"IMD-202"), "{codes:?}");
        assert!(a.markdown.contains("id=\"imd-diag\""));
        assert!(a.markdown.contains("imd-chip imd-zly"));
        // The unclosed block opens no frame.
        assert!(!a.markdown.contains("imd-blok"));
        assert!(a.markdown.contains("błędy: 3"));
    }

    #[test]
    fn test_vocabulary_labels_and_unknown_names() {
        let v = Vocabulary::parse(
            r#"{"standard": "ITA-MD 1.0", "nazwa": "test", "fakty": {"tier": {"typ": "poziom", "etykieta": "Poziom", "opis": "Poziom usług"}}}"#,
        )
        .unwrap();
        let a = annotate(
            "> 🔀 ⟦JEŻELI tier >= T4 and api⟧\n\n6. x\n> 🔀 ⟦KONIEC⟧\n",
            &AnnotateOptions { vocabulary: Some(&v), ..Default::default() },
        );
        assert!(a.markdown.contains("<b>JEŻELI</b> Poziom ≥ T4 i api"), "{}", a.markdown);
        assert!(a.markdown.contains("tier: Poziom usług"));
        assert!(a.markdown.contains("słownik: test"));
        assert_eq!(a.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(), ["IMD-402"]);
    }

    #[test]
    fn test_bare_mode_and_preview_helpers() {
        let src = "> 🔀 ⟦JEŻELI ef⟧\n\n6. x\n> 🔀 ⟦KONIEC⟧\n";
        let bare =annotate(src, &AnnotateOptions { bare: true, ..Default::default() });
        assert!(!bare.markdown.contains("imd-pasek") && bare.markdown.contains("imd-blok"));
        let full = ann(src);
        assert!(full.markdown.contains("data-imd-akcja=\"profil\""));
        let ok = preview_toolbar("profil-T3.json", Ok(4));
        assert!(ok.contains("profil-T3.json") && ok.contains("zaślepki: 4") && ok.contains("data-imd-akcja=\"zrodlo\""));
        assert!(preview_toolbar("x.json", Err(2)).contains("podgląd niemożliwy — błędy: 2"));
        assert_eq!(
            dim_stubs("<p>6. Nie dotyczy niniejszej Umowy.</p><td>nie dotyczy niniejszej Umowy</td>"),
            "<p>6. <span class=\"imd-nd\">Nie dotyczy niniejszej Umowy.</span></p><td><span class=\"imd-nd\">nie dotyczy niniejszej Umowy</span></td>"
        );
        let panel = diagnostics_panel(&[Diagnostic::new("IMD-503", 0, "brak faktu „x”")]);
        assert!(panel.contains("IMD-503") && panel.contains("id=\"imd-diag\""));
    }

    #[test]
    fn test_escaping_keeps_markdown_and_tables_intact() {
        assert_eq!(esc("a|b*c_d<e>&\"[x]`~\\"), "a&#124;b&#42;c&#95;d&lt;e&gt;&amp;&quot;&#91;x&#93;&#96;&#126;&#92;");
    }

    #[test]
    fn test_crlf_source() {
        let a = ann("> 🔀 ⟦JEŻELI a⟧\r\n\r\n6. x\r\n> 🔀 ⟦KONIEC⟧\r\n");
        assert!(a.markdown.contains("\r\n"));
        let html = markdown_to_html(&a.markdown, None);
        assert!(html.contains("imd-blok") && html.contains("</div></div>"), "{html}");
    }

    #[test]
    fn test_css_has_both_palettes_and_hide_rules() {
        let light = page_css(false, "#e1e4e8");
        let dark = page_css(true, "#444");
        assert!(light.contains("--imd-if:#0969da") && dark.contains("--imd-if:#4493f8"));
        assert!(light.contains(".imd-ukryj .imd-gal") && light.contains("#e1e4e8"));
        assert!(PAGE_JS.contains("itamd-ukryj") && PAGE_JS.contains("localStorage"));
    }
}
