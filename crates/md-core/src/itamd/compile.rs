//! Compile a source for one configuration: evaluate every directive against
//! a fact set and produce the deliverable's text.
//!
//! Passes, in the order the standard fixes: blocks → rows → tier-column
//! tables → inline → clean-up. A block that does not apply leaves a stub
//! (`numer`, `tytuł`) or nothing (`brak`); a row is kept, dropped, or
//! stubbed in a cell; a table under `TABELA · kolumny: poziom` keeps its
//! first column, columns without a tier name, and the configuration's tier
//! column. The compiler fails closed: any error in the source, in the fact
//! set, or during evaluation stops it, and nothing is produced.

use super::analyze::{self, heading, head_line, item, split_cells, tier_in, BlockNode, BlockRole, Node, TokenRole};
use super::diag::Diagnostic;
use super::expr::{Facts, Value};
use super::lex::{self, Directive, Keyword, LineKind, Stub, Token};
use super::vocab::Vocabulary;
use super::{sort, STUB_BLOCK, STUB_CELL};
use crate::source::{self, Layout};

/// The fact `TABELA · kolumny: poziom` reads: the configuration's tier.
pub const TIER_FACT: &str = "tier";

/// What [`compile`] needs besides the source and the facts.
#[derive(Default)]
pub struct CompileOptions<'a> {
    /// The project's vocabulary: when given, the fact set is validated
    /// against it (missing, extra, mistyped facts are errors) and every name
    /// in the source must belong to it.
    pub vocabulary: Option<&'a Vocabulary>,
}

/// A compiled deliverable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiled {
    /// The deliverable's text (LF line endings, one final newline).
    pub text: String,
    /// Stubs left by blocks and rows that do not apply.
    pub stubs: usize,
}

/// Compile `source` for the configuration described by `facts`.
pub fn compile(source: &str, facts: &Facts, opts: &CompileOptions) -> Result<Compiled, Vec<Diagnostic>> {
    let (text, stubs) = project(source, facts, opts)?;
    let text = cleanup(&text);
    if let Some(no) = leftover_line(&text) {
        return Err(vec![Diagnostic::new(
            "IMD-603",
            no,
            "po kompilacji w tekście (poza kodem) została dyrektywa albo jej nawias; wynik odrzucony",
        )]);
    }
    Ok(Compiled { text, stubs })
}

/// Self-check of a compiled text: the 1-based line of the first leftover
/// bracket or 🔀 line outside code, if any.
pub(crate) fn leftover_line(text: &str) -> Option<usize> {
    let layout = Layout::of(text);
    source::lines(text).iter().enumerate().find_map(|(i, l)| {
        let lead = l.text.len() - l.text.trim_start().len();
        if layout.is_block_excluded(l.start + lead) {
            return None;
        }
        let bracket = l.text.char_indices().any(|(k, c)| (c == '⟦' || c == '⟧') && !layout.is_code(l.start + k));
        let note = l.text[lead..]
            .strip_prefix('>')
            .is_some_and(|r| r.trim_start().starts_with(lex::DIRECTIVE_EMOJI));
        (bracket || note).then_some(i + 1)
    })
}

/// The passes without the final clean-up: the text exactly as the passes
/// leave it (LF line endings), plus the number of stubs. Exposed so a
/// project with its own post-processing can run it before [`cleanup`].
pub fn project(source: &str, facts: &Facts, opts: &CompileOptions) -> Result<(String, usize), Vec<Diagnostic>> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    // Line endings are normalised first: the deliverable is LF text.
    let normalized;
    let source = if source.contains('\r') {
        normalized = source.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        source
    };

    let scan = lex::scan(source);
    let analysis = analyze::analyze(&scan, opts.vocabulary);
    let mut problems: Vec<Diagnostic> =
        scan.diagnostics.iter().chain(&analysis.diagnostics).filter(|d| d.is_error()).cloned().collect();
    if let Some(v) = opts.vocabulary {
        problems.extend(v.validate(facts));
    }
    if !problems.is_empty() {
        sort(&mut problems);
        return Err(problems);
    }

    let mut ev = Evaluator { facts, errors: Vec::new(), stubs: 0 };

    // Pass 1: blocks.
    let mut lines: Vec<Out> = Vec::new();
    ev.blocks(&scan, &analysis.tree, &mut lines);

    // Pass 2: rows.
    let mut rowed: Vec<Out> = Vec::with_capacity(lines.len());
    for line in lines {
        match line {
            Out::Source(i) => match row_token(&scan, &analysis.token_roles, i) {
                Some(d) => {
                    if let Some(text) = ev.row(&scan, i, d) {
                        rowed.push(Out::Text(text));
                    }
                }
                None => rowed.push(Out::Source(i)),
            },
            other => rowed.push(other),
        }
    }

    // Pass 3: tier-column tables.
    let text_of = |o: &Out| -> String {
        match o {
            Out::Source(i) => scan.lines[*i].text.to_string(),
            Out::Text(t) => t.clone(),
        }
    };
    let mut tabled: Vec<String> = Vec::with_capacity(rowed.len());
    let mut k = 0;
    while k < rowed.len() {
        let is_table = matches!(rowed[k], Out::Source(i) if analysis.block_roles[i] == Some(BlockRole::Table));
        if !is_table {
            tabled.push(text_of(&rowed[k]));
            k += 1;
            continue;
        }
        k += 1;
        while k < rowed.len() && text_of(&rowed[k]).trim().is_empty() {
            k += 1;
        }
        let mut table = Vec::new();
        while k < rowed.len() && text_of(&rowed[k]).trim_start().starts_with('|') {
            table.push(text_of(&rowed[k]));
            k += 1;
        }
        match ev.tier() {
            Some(tier) => tabled.extend(project_tier_columns(&table, &tier)),
            None => tabled.extend(table),
        }
    }

    // Pass 4: inline directives, on the text the earlier passes produced.
    let mut joined = tabled.join("\n");
    if source.ends_with('\n') {
        joined.push('\n');
    }
    let inter = lex::scan(&joined);
    let mut out = String::with_capacity(joined.len());
    for (line, kind) in inter.lines.iter().zip(&inter.kinds) {
        match kind {
            LineKind::Text { tokens } if tokens.iter().any(|t| matches!(t, Token::Directive(_))) => {
                out.push_str(&ev.inline(line.text, tokens));
            }
            _ => out.push_str(line.text),
        }
        out.push_str(line.ending);
    }

    if !ev.errors.is_empty() {
        sort(&mut ev.errors);
        ev.errors.dedup();
        return Err(ev.errors);
    }
    Ok((out, ev.stubs))
}

/// A line of the text being compiled: still the source line, or rewritten.
enum Out {
    Source(usize),
    Text(String),
}

/// The well-placed row directive of source line `i`, if any.
fn row_token<'a>(scan: &'a lex::Scan, roles: &[Vec<TokenRole>], i: usize) -> Option<&'a Directive> {
    let LineKind::Text { tokens } = &scan.kinds[i] else { return None };
    tokens.iter().zip(&roles[i]).find_map(|(t, r)| match (t, r) {
        (Token::Directive(d), TokenRole::Row) => Some(d),
        _ => None,
    })
}

struct Evaluator<'a> {
    facts: &'a Facts,
    errors: Vec<Diagnostic>,
    stubs: usize,
}

impl Evaluator<'_> {
    /// Evaluate a directive's condition; an evaluation error is recorded
    /// and counts as false (the compilation fails anyway).
    fn eval(&mut self, d: &Directive) -> bool {
        let Some(e) = &d.expr else { return false };
        match e.eval(self.facts) {
            Ok(b) => b,
            Err(err) => {
                self.errors.push(err.diagnostic(d.line, &d.expr_text));
                false
            }
        }
    }

    fn tier(&mut self) -> Option<String> {
        match self.facts.get(TIER_FACT) {
            Some(Value::Text(t)) => Some(t.clone()),
            other => {
                let why = match other {
                    None => format!("brak faktu „{TIER_FACT}”"),
                    Some(_) => format!("fakt „{TIER_FACT}” nie jest tekstem"),
                };
                self.errors.push(Diagnostic::new(
                    "IMD-503",
                    0,
                    format!("dyrektywa TABELA czyta Poziom konfiguracji z faktu „{TIER_FACT}”: {why}"),
                ));
                None
            }
        }
    }

    /// Pass 1: the first true branch, else the W PRZECIWNYM RAZIE branch,
    /// else the stub.
    fn blocks(&mut self, scan: &lex::Scan, nodes: &[Node], out: &mut Vec<Out>) {
        for node in nodes {
            match node {
                Node::Line(i) => out.push(Out::Source(*i)),
                Node::Block(b) => {
                    let mut chosen: Option<&[Node]> = None;
                    for (line, children) in &b.branches {
                        let d = analyze::block_directive(&scan.kinds[*line]).expect("branch line is a block line");
                        if self.eval(d) {
                            chosen = Some(children);
                            break;
                        }
                    }
                    if chosen.is_none() {
                        chosen = b.otherwise.as_ref().map(|(_, c)| c.as_slice());
                    }
                    match chosen {
                        Some(children) => self.blocks(scan, children, out),
                        None => {
                            let stub = make_stub(scan, b);
                            if !stub.is_empty() {
                                self.stubs += 1;
                            }
                            out.extend(stub.into_iter().map(Out::Text));
                        }
                    }
                }
            }
        }
    }

    /// Pass 2: a row that applies loses its directive; one that does not
    /// is dropped (`brak`, the default) or stubbed in a cell.
    fn row(&mut self, scan: &lex::Scan, i: usize, d: &Directive) -> Option<String> {
        let text = scan.lines[i].text;
        // The directive goes together with the whitespace after it.
        let after = text[d.span.end..].trim_start();
        let clean = format!("{}{}", &text[..d.span.start], after).replace("|  ", "| ");
        if self.eval(d) {
            return Some(clean);
        }
        let keep = match d.stub {
            Some(Stub::Number) => 1,
            Some(Stub::Title) => 2,
            _ => return None,
        };
        let cells = split_cells(&clean);
        let keep = if keep == 2 && cells.len() > 2 { 2 } else { 1 };
        let mut row: Vec<&str> = cells.iter().take(keep).copied().collect();
        row.push(STUB_CELL);
        row.extend(std::iter::repeat_n("", cells.len().saturating_sub(keep + 1)));
        self.stubs += 1;
        Some(join_row(&row))
    }

    /// Pass 4: inline groups, left to right with a stack; a fragment is kept
    /// only when every open condition above it is true.
    fn inline(&mut self, line: &str, tokens: &[Token]) -> String {
        // Stack entries: [parent active, a branch already taken, this branch active].
        let mut stack: Vec<(bool, bool, bool)> = Vec::new();
        let mut out = String::with_capacity(line.len());
        let mut pos = 0;
        let active = |s: &[(bool, bool, bool)]| s.iter().all(|e| e.2);
        for tok in tokens {
            let Token::Directive(d) = tok else { continue };
            if matches!(d.keyword, Keyword::Row | Keyword::Table) {
                continue;
            }
            if active(&stack) {
                out.push_str(&line[pos..d.span.start]);
            }
            pos = d.span.end;
            let parent = active(&stack);
            match d.keyword {
                Keyword::If => {
                    let ok = self.eval(d);
                    stack.push((parent, ok, ok));
                }
                Keyword::ElseIf => {
                    if let Some(top) = stack.last().copied() {
                        let ok = !top.1 && self.eval(d);
                        let last = stack.last_mut().unwrap();
                        last.2 = ok;
                        last.1 = top.1 || ok;
                    }
                }
                Keyword::Else => {
                    if let Some(top) = stack.last_mut() {
                        top.2 = !top.1;
                        top.1 = true;
                    }
                }
                Keyword::End => {
                    stack.pop();
                }
                Keyword::Row | Keyword::Table => unreachable!(),
            }
        }
        if active(&stack) {
            out.push_str(&line[pos..]);
        }
        out
    }
}

/// `| a | b |` from cells, as the reference generator writes rows.
fn join_row(cells: &[&str]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// The stub of a block that does not apply: `numer` keeps the unit's
/// designation, `tytuł` the designation with the title, `brak` nothing.
fn make_stub(scan: &lex::Scan, b: &BlockNode) -> Vec<String> {
    if b.stub == Stub::None {
        return Vec::new();
    }
    let head = head_line(scan, &b.branches[0].1);
    if let Some((level, text)) = heading(&head) {
        let hashes = "#".repeat(level);
        let title = match (b.stub, heading_designation(text)) {
            (Stub::Number, Some(des)) => format!("{hashes} {des}"),
            _ => head.clone(),
        };
        return vec![title, String::new(), STUB_BLOCK.to_string()];
    }
    if let Some((indent, des, rest)) = item(&head) {
        let lead = if b.stub == Stub::Title { bold_lead(rest).map(|l| format!("{l} ")).unwrap_or_default() } else { String::new() };
        return vec![format!("{indent}{des} {lead}{STUB_BLOCK}")];
    }
    // No designation (IMD-602 stops this in a checked source).
    vec![format!("{}{STUB_BLOCK}", " ".repeat(b.indent))]
}

/// `^((?:§\s?)?[A-Z]?\d+[a-z]?(?:\.\d+)?\.?)(\s|$)`: the designation that
/// opens a heading's text (`E10.`, `§ 1a.`, `4.`, `E5.1`), if any.
pub(crate) fn heading_designation(text: &str) -> Option<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let at = |k: usize| chars.get(k).map(|&(_, c)| c);
    let end_at = |k: usize| chars.get(k).map(|&(i, _)| i).unwrap_or(text.len());
    let ends_ok = |k: usize| at(k).is_none_or(char::is_whitespace);
    // Alternatives in the regex's (greedy-first) order.
    let mut prefixes = Vec::new();
    if at(0) == Some('§') {
        if at(1).is_some_and(char::is_whitespace) {
            prefixes.push(2);
        }
        prefixes.push(1);
    } else {
        prefixes.push(0);
    }
    for p in prefixes {
        let uppers: Vec<usize> = if at(p).is_some_and(|c| c.is_ascii_uppercase()) { vec![p + 1, p] } else { vec![p] };
        for u in uppers {
            let mut d = u;
            while at(d).is_some_and(|c| c.is_ascii_digit()) {
                d += 1;
            }
            if d == u {
                continue;
            }
            let lowers: Vec<usize> = if at(d).is_some_and(|c| c.is_ascii_lowercase()) { vec![d + 1, d] } else { vec![d] };
            for l in lowers {
                let mut subs = Vec::new();
                if at(l) == Some('.') && at(l + 1).is_some_and(|c| c.is_ascii_digit()) {
                    let mut s = l + 1;
                    while at(s).is_some_and(|c| c.is_ascii_digit()) {
                        s += 1;
                    }
                    subs.push(s);
                }
                subs.push(l);
                for s in subs {
                    let dots: Vec<usize> = if at(s) == Some('.') { vec![s + 1, s] } else { vec![s] };
                    for e in dots {
                        if ends_ok(e) {
                            return Some(&text[..end_at(e)]);
                        }
                    }
                }
            }
        }
    }
    None
}

/// `^(\*\*[^*]+\*\*)`: a bold label opening an item's text.
fn bold_lead(rest: &str) -> Option<&str> {
    let inner = rest.strip_prefix("**")?;
    let len = inner.find('*')?;
    (len > 0 && inner[len..].starts_with("**")).then(|| &rest[..len + 4])
}

/// Pass 3 on one table: keep column 0, columns without a tier in the
/// header, and the column of `tier`; drop rows whose own-tier cell is empty
/// (`—`, `-`, `–`, optionally followed by a parenthesised note).
fn project_tier_columns(table: &[String], tier: &str) -> Vec<String> {
    if table.len() < 2 {
        return table.to_vec();
    }
    let header = split_cells(&table[0]);
    let keep: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(idx, cell)| *idx == 0 || tier_in(cell).is_none_or(|t| t == tier))
        .map(|(idx, _)| idx)
        .collect();
    let own = keep.iter().copied().find(|&k| k > 0 && tier_in(header[k]).is_some());
    let pick = |cells: &[&str]| -> String {
        let row: Vec<&str> = keep.iter().map(|&k| cells.get(k).copied().unwrap_or("")).collect();
        join_row(&row)
    };
    let mut out = vec![pick(&header), pick(&split_cells(&table[1]))];
    for row in &table[2..] {
        let cells = split_cells(row);
        if let Some(o) = own {
            if cells.get(o).is_some_and(|c| is_empty_cell(c)) {
                continue;
            }
        }
        out.push(pick(&cells));
    }
    out
}

/// `^(—|-|–)(\s*\(.*\))?$`
fn is_empty_cell(cell: &str) -> bool {
    let Some(rest) = cell.strip_prefix(['—', '-', '–']) else { return false };
    let rest = rest.trim_start();
    rest.is_empty() || (rest.starts_with('(') && rest.ends_with(')'))
}

/// The standard clean-up after the passes (code blocks untouched):
///
/// * a horizontal rule `---` separated from the previous one only by blank
///   lines is dropped together with those lines (removed sections must not
///   leave rules stacked up);
/// * runs of empty lines become one empty line;
/// * leading and trailing whitespace and trailing rules are removed, and the
///   text ends with exactly one newline.
pub fn cleanup(text: &str) -> String {
    let layout = Layout::of(text);
    let lines = source::lines(text);
    let in_code = |l: &source::SourceLine| {
        let lead = l.text.len() - l.text.trim_start().len();
        layout.is_block_excluded(l.start + lead)
    };
    // Rules: drop `---` (plus the blank lines before it) when the previous
    // non-blank line is also `---` and at least one blank line separates them.
    let mut kept: Vec<(&str, bool)> = Vec::with_capacity(lines.len());
    for l in &lines {
        let code = in_code(l);
        if !code && l.text == "---" {
            let blanks = kept.iter().rev().take_while(|(t, c)| !c && t.trim().is_empty()).count();
            let before = kept.len() - blanks;
            if blanks > 0 && before > 0 && kept[before - 1] == ("---", false) {
                kept.truncate(before);
                continue;
            }
        }
        kept.push((l.text, code));
    }
    // Empty-line runs.
    let mut out_lines: Vec<&str> = Vec::with_capacity(kept.len());
    let mut prev_empty = false;
    for (t, code) in kept {
        let empty = !code && t.is_empty();
        if empty && prev_empty {
            continue;
        }
        prev_empty = empty;
        out_lines.push(t);
    }
    let mut s = out_lines.join("\n");
    s = s.trim().to_string();
    // Trailing rules (with the whitespace around them).
    loop {
        let trimmed = s.trim_end();
        match trimmed.rfind('\n') {
            Some(nl) if trimmed[nl + 1..].trim() == "---" => s = trimmed[..nl].to_string(),
            _ => break,
        }
    }
    let mut s = s.trim().to_string();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::itamd::parse_facts;

    fn facts(json: &str) -> Facts {
        parse_facts(json).unwrap()
    }

    fn run(src: &str, json: &str) -> String {
        compile(src, &facts(json), &CompileOptions::default()).unwrap().text
    }

    fn fail(src: &str, json: &str) -> Vec<&'static str> {
        compile(src, &facts(json), &CompileOptions::default()).unwrap_err().iter().map(|d| d.code).collect()
    }

    #[test]
    fn test_first_true_branch_else_branch_else_stub() {
        let src = "> 🔀 ⟦JEŻELI tier == T5⟧\n\n6. Piąty.\n> 🔀 ⟦ALBO JEŻELI tier == T4⟧\n\n6. Czwarty.\n> 🔀 ⟦W PRZECIWNYM RAZIE⟧\n\n6. Inny.\n> 🔀 ⟦KONIEC⟧\n";
        assert_eq!(run(src, r#"{"tier": "T5"}"#), "6. Piąty.\n");
        assert_eq!(run(src, r#"{"tier": "T4"}"#), "6. Czwarty.\n");
        assert_eq!(run(src, r#"{"tier": "T1"}"#), "6. Inny.\n");
        let no_else = "> 🔀 ⟦JEŻELI tier == T5⟧\n\n6. **Piąty.** treść\n> 🔀 ⟦KONIEC⟧\n";
        assert_eq!(run(no_else, r#"{"tier": "T1"}"#), "6. Nie dotyczy niniejszej Umowy.\n");
    }

    #[test]
    fn test_stub_styles() {
        let item = |stub: &str| format!("> 🔀 ⟦JEŻELI ef{stub}⟧\n\n   a. **Tytuł punktu.** treść\n> 🔀 ⟦KONIEC⟧\n\nPo.\n");
        let f = r#"{"ef": false}"#;
        assert_eq!(run(&item(""), f), "a. Nie dotyczy niniejszej Umowy.\nPo.\n");
        assert_eq!(run(&item(" · zaślepka: tytuł"), f), "a. **Tytuł punktu.** Nie dotyczy niniejszej Umowy.\nPo.\n");
        assert_eq!(run(&item(" · zaślepka: brak"), f), "Po.\n");
        let head = |stub: &str| format!("Przed.\n\n> 🔀 ⟦JEŻELI ef{stub}⟧\n\n## E10. Tryb CITO\n\nTreść.\n> 🔀 ⟦KONIEC⟧\n\n\nPo.\n");
        assert_eq!(run(&head(""), f), "Przed.\n\n## E10.\n\nNie dotyczy niniejszej Umowy.\n\nPo.\n");
        assert_eq!(run(&head(" · zaślepka: tytuł"), f), "Przed.\n\n## E10. Tryb CITO\n\nNie dotyczy niniejszej Umowy.\n\nPo.\n");
        let par = "### § 1a. Zastąpienie\n";
        assert_eq!(heading_designation(&par[4..]), Some("§ 1a."));
    }

    #[test]
    fn test_heading_designations() {
        for (t, d) in [
            ("E10. Tryb", Some("E10.")),
            ("§ 1a. X", Some("§ 1a.")),
            ("§1a. X", Some("§1a.")),
            ("4. Utrzymanie", Some("4.")),
            ("E5.1 Święta", Some("E5.1")),
            ("E16.3. Wykup", Some("E16.3.")),
            ("12", Some("12")),
            ("Arkusze szczegółowe", None),
            ("1.2.3 x", None),
            ("T5-A", None),
        ] {
            assert_eq!(heading_designation(t), d, "{t}");
        }
        assert_eq!(bold_lead("**F7** reszta"), Some("**F7**"));
        assert_eq!(bold_lead("**** x"), None);
        assert_eq!(bold_lead("zwykły"), None);
    }

    #[test]
    fn test_rows() {
        let src = "| Poz. | Opis | Opłata |\n|---|---|---|\n| ⟦WIERSZ holidays · zaślepka: numer⟧**E5.1** | Święta | +20% |\n| ⟦WIERSZ standby24 · zaślepka: tytuł⟧**E5.2** | Dyżur | +20% |\n| ⟦WIERSZ standby24⟧**E5.3** | Okno | £250 |\n";
        let all = run(src, r#"{"holidays": true, "standby24": true}"#);
        assert_eq!(all, "| Poz. | Opis | Opłata |\n|---|---|---|\n| **E5.1** | Święta | +20% |\n| **E5.2** | Dyżur | +20% |\n| **E5.3** | Okno | £250 |\n");
        let none = run(src, r#"{"holidays": false, "standby24": false}"#);
        assert_eq!(
            none,
            "| Poz. | Opis | Opłata |\n|---|---|---|\n| **E5.1** | nie dotyczy niniejszej Umowy |  |\n| **E5.2** | Dyżur | nie dotyczy niniejszej Umowy |\n"
        );
    }

    #[test]
    fn test_tier_tables() {
        let src = "> 🔀 ⟦TABELA · kolumny: poziom⟧\n\n| Wymiar | **T1 Essential** | **T2 On-demand** | **T3 Priority** |\n|---|---|---|---|\n| Naprawy | w miarę | w miarę | priorytetowo |\n| Pula | — | — | h/mc |\n| Standby | — (brak) | — | tak |\n";
        assert_eq!(
            run(src, r#"{"tier": "T2"}"#),
            "| Wymiar | **T2 On-demand** |\n| --- | --- |\n| Naprawy | w miarę |\n"
        );
        assert_eq!(
            run(src, r#"{"tier": "T3"}"#),
            "| Wymiar | **T3 Priority** |\n| --- | --- |\n| Naprawy | priorytetowo |\n| Pula | h/mc |\n| Standby | tak |\n"
        );
        assert_eq!(fail(src, "{}"), ["IMD-503"]);
    }

    #[test]
    fn test_inline() {
        let src = "Czas ⟦JEŻELI tier >= T4⟧określają Załączniki A i D⟦W PRZECIWNYM RAZIE⟧określa Załącznik A⟦KONIEC⟧.\n";
        assert_eq!(run(src, r#"{"tier": "T4"}"#), "Czas określają Załączniki A i D.\n");
        assert_eq!(run(src, r#"{"tier": "T2"}"#), "Czas określa Załącznik A.\n");
        let nested = "Rozwój.⟦JEŻELI tier >= T3⟧ Z puli (⟦JEŻELI tier >= T4⟧A/D⟦W PRZECIWNYM RAZIE⟧A⟦KONIEC⟧).⟦KONIEC⟧\n";
        assert_eq!(run(nested, r#"{"tier": "T5"}"#), "Rozwój. Z puli (A/D).\n");
        assert_eq!(run(nested, r#"{"tier": "T3"}"#), "Rozwój. Z puli (A).\n");
        assert_eq!(run(nested, r#"{"tier": "T1"}"#), "Rozwój.\n");
        let elif = "Osoby: ⟦JEŻELI tier == T1⟧1⟦ALBO JEŻELI tier == T2⟧2⟦W PRZECIWNYM RAZIE⟧3+⟦KONIEC⟧.\n";
        assert_eq!(run(elif, r#"{"tier": "T2"}"#), "Osoby: 2.\n");
        assert_eq!(run(elif, r#"{"tier": "T5"}"#), "Osoby: 3+.\n");
    }

    #[test]
    fn test_pass_order_row_inside_false_block_disappears_with_it() {
        let src = "> 🔀 ⟦JEŻELI ef⟧\n\n## E12. Siatka\n\n| Plan | Kwota |\n|---|---|\n| ⟦WIERSZ api⟧E0 | £2 |\n> 🔀 ⟦KONIEC⟧\n\n\nKoniec.\n";
        // `api` is never read: the block is false first.
        assert_eq!(run(src, r#"{"ef": false}"#), "## E12.\n\nNie dotyczy niniejszej Umowy.\n\nKoniec.\n");
        assert_eq!(fail(src, r#"{"ef": true}"#), ["IMD-503"]);
    }

    #[test]
    fn test_fail_closed() {
        // Source errors stop compilation before any evaluation.
        assert_eq!(fail("> 🔀 ⟦JEŻELI a⟧\n\ntekst\n", r#"{"a": true}"#), ["IMD-101"]);
        assert_eq!(fail("> 🔀 ⟦JEŻELI a⟧\n\nZwykły akapit.\n> 🔀 ⟦KONIEC⟧\n", r#"{"a": true}"#), ["IMD-602"]);
        assert_eq!(fail("a ⟦JEŻELI x⟧b⟦KONIEC⟧\n", "{}"), ["IMD-503"]);
        assert_eq!(fail("a ⟦JEŻELI x > T1⟧b⟦KONIEC⟧\n", r#"{"x": "L1"}"#), ["IMD-604"]);
        // With a vocabulary the fact set itself is validated.
        let v = Vocabulary::parse(r#"{"standard": "ITA-MD 1.0", "fakty": {"ef": {"typ": "logiczny"}}}"#).unwrap();
        let err = compile("a\n", &facts(r#"{"ef": "tak", "x": true}"#), &CompileOptions { vocabulary: Some(&v) }).unwrap_err();
        assert_eq!(err.iter().map(|d| d.code).collect::<Vec<_>>(), ["IMD-504", "IMD-505"]);
    }

    #[test]
    fn test_layout_equals_source_without_directives() {
        let src = "Przed.\n\n> 🔀 ⟦JEŻELI ef · zaślepka: brak⟧\n\nŚrodek.\n> 🔀 ⟦KONIEC⟧\n\n\nPo.\n";
        assert_eq!(run(src, r#"{"ef": true}"#), "Przed.\n\nŚrodek.\n\nPo.\n");
        assert_eq!(run(src, r#"{"ef": false}"#), "Przed.\n\nPo.\n");
    }

    #[test]
    fn test_leftover_self_check() {
        assert_eq!(leftover_line("a\nb ⟦ c\n"), Some(2));
        assert_eq!(leftover_line("> 🔀 x\n"), Some(1));
        assert_eq!(leftover_line("`⟦x⟧`\n\n```\n> 🔀 ⟦KONIEC⟧\n```\n"), None);
    }

    #[test]
    fn test_cleanup() {
        assert_eq!(cleanup("\n\na\n\n\n\nb\n---\n\n---\n\nc\n\n---\n\n"), "a\n\nb\n---\n\nc\n");
        assert_eq!(cleanup("x\n---\n---\ny"), "x\n---\n---\ny\n"); // adjacent rules are not merged
        assert_eq!(cleanup("```\na\n\n\n\nb\n```\n"), "```\na\n\n\n\nb\n```\n");
    }

    #[test]
    fn test_crlf_and_code_untouched() {
        let src = "a ⟦JEŻELI ef⟧b⟦KONIEC⟧\r\n\r\n```\n⟦JEŻELI ef⟧ w przykładzie\n```\r\n";
        assert_eq!(run(src, r#"{"ef": false}"#), "a \n\n```\n⟦JEŻELI ef⟧ w przykładzie\n```\n");
    }

    #[test]
    fn test_stub_count() {
        let src = "> 🔀 ⟦JEŻELI ef⟧\n\n6. x\n> 🔀 ⟦KONIEC⟧\n\n| P | Q |\n|---|---|\n| ⟦WIERSZ ef · zaślepka: numer⟧a | b |\n";
        let c = compile(src, &facts(r#"{"ef": false}"#), &CompileOptions::default()).unwrap();
        assert_eq!(c.stubs, 2);
    }
}
