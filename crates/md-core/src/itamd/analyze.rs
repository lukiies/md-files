//! Structure: pair block directives into blocks (a tree, as a compiler sees
//! it), pair inline directives into groups, check where row and table
//! directives stand, check expressions, units of stubbed blocks, and
//! leftovers of the provisional HTML markers.

use super::diag::Diagnostic;
use super::expr;
use super::lex::{Directive, Keyword, LineKind, Scan, Stub, Token};
use super::vocab::Vocabulary;

/// The part a block directive line plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockRole {
    /// Opens block `id` (its first branch).
    Open(usize),
    /// Starts another branch of block `id` (ALBO JEŻELI, W PRZECIWNYM RAZIE).
    Branch(usize),
    /// Closes block `id`.
    Close(usize),
    /// A TABELA line.
    Table,
    /// Part of a structurally broken block (reported).
    Unmatched,
}

/// The part an inline token plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenRole {
    Open,
    Branch,
    Close,
    /// A well-placed row directive.
    Row,
    /// Not usable: lexically bad, misplaced, or in a broken group.
    Broken,
}

/// A node of the block tree: a source line (by index) or a block.
#[derive(Debug, Clone)]
pub(crate) enum Node {
    Line(usize),
    Block(BlockNode),
}

/// One conditional block.
#[derive(Debug, Clone)]
pub(crate) struct BlockNode {
    /// Line index of the JEŻELI line.
    pub open: usize,
    /// Indentation of the JEŻELI line (bytes of spaces).
    pub indent: usize,
    /// The stub style of the block (first branch's option, default `numer`).
    pub stub: Stub,
    /// JEŻELI and ALBO JEŻELI branches: the directive line and the content.
    pub branches: Vec<(usize, Vec<Node>)>,
    /// The W PRZECIWNYM RAZIE branch, if any.
    pub otherwise: Option<(usize, Vec<Node>)>,
    /// Line index of the KONIEC line (`None` while unclosed).
    pub close: Option<usize>,
}

impl BlockNode {
    fn target(&mut self) -> &mut Vec<Node> {
        match &mut self.otherwise {
            Some((_, nodes)) => nodes,
            None => &mut self.branches.last_mut().unwrap().1,
        }
    }
}

/// Counts shown by viewers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Block directives (JEŻELI … KONIEC).
    pub blocks: usize,
    /// Inline directive groups.
    pub inline: usize,
    /// Row directives.
    pub rows: usize,
    /// Table directives.
    pub tables: usize,
}

impl Stats {
    /// True if the source holds any directive at all.
    pub fn any(&self) -> bool {
        self.blocks + self.inline + self.rows + self.tables > 0
    }
}

pub(crate) struct Analysis {
    pub block_roles: Vec<Option<BlockRole>>,
    pub token_roles: Vec<Vec<TokenRole>>,
    pub tree: Vec<Node>,
    pub stats: Stats,
    pub diagnostics: Vec<Diagnostic>,
}

/// The directive of a block line, if `kind` is one.
pub(crate) fn block_directive(kind: &LineKind) -> Option<&Directive> {
    match kind {
        LineKind::Block { directive, .. } => Some(directive),
        _ => None,
    }
}

pub(crate) fn analyze(scan: &Scan, vocab: Option<&Vocabulary>) -> Analysis {
    let n = scan.lines.len();
    let mut diags = Vec::new();
    let mut block_roles: Vec<Option<BlockRole>> = vec![None; n];
    let mut stats = Stats::default();

    // ---- blocks: roles, tree, structure errors ---------------------------
    let mut root: Vec<Node> = Vec::new();
    let mut stack: Vec<(usize, BlockNode)> = Vec::new(); // (block id, node)
    let mut next_id = 0;
    let mut skip_blank = false;
    let mut branch_lines: Vec<Vec<usize>> = Vec::new(); // per block id: lines with roles to undo if unclosed
    for (idx, (line, kind)) in scan.lines.iter().zip(&scan.kinds).enumerate() {
        if std::mem::take(&mut skip_blank) && line.is_blank() {
            continue;
        }
        let (indent, d) = match kind {
            LineKind::Block { indent, directive, .. } => (*indent, directive),
            _ => {
                push_node(&mut root, &mut stack, Node::Line(idx));
                continue;
            }
        };
        let no = idx + 1;
        match d.keyword {
            Keyword::Table => {
                block_roles[idx] = Some(BlockRole::Table);
                stats.tables += 1;
                push_node(&mut root, &mut stack, Node::Line(idx));
                continue;
            }
            Keyword::If => {
                let id = next_id;
                next_id += 1;
                stats.blocks += 1;
                block_roles[idx] = Some(BlockRole::Open(id));
                branch_lines.push(vec![idx]);
                let stub = d.stub.unwrap_or(Stub::Number);
                stack.push((
                    id,
                    BlockNode { open: idx, indent, stub, branches: vec![(idx, Vec::new())], otherwise: None, close: None },
                ));
            }
            Keyword::ElseIf | Keyword::Else => {
                let Some((id, top)) = stack.last_mut() else {
                    diags.push(Diagnostic::new(
                        "IMD-103",
                        no,
                        format!("{} poza blokiem: brak otwartego ⟦JEŻELI⟧ w linii blokowej", d.keyword.word()),
                    ));
                    block_roles[idx] = Some(BlockRole::Unmatched);
                    skip_blank = true;
                    continue;
                };
                if top.otherwise.is_some() {
                    diags.push(Diagnostic::new(
                        "IMD-104",
                        no,
                        format!(
                            "{} po ⟦W PRZECIWNYM RAZIE⟧ bloku z linii {}; gałąź „w przeciwnym razie” jest ostatnia",
                            d.keyword.word(),
                            top.open + 1
                        ),
                    ));
                    block_roles[idx] = Some(BlockRole::Unmatched);
                    skip_blank = true;
                    continue;
                }
                if indent != top.indent {
                    diags.push(indent_warning(d, indent, top));
                }
                block_roles[idx] = Some(BlockRole::Branch(*id));
                branch_lines[*id].push(idx);
                if d.keyword == Keyword::ElseIf {
                    top.branches.push((idx, Vec::new()));
                } else {
                    top.otherwise = Some((idx, Vec::new()));
                }
            }
            Keyword::End => {
                let Some((id, mut top)) = stack.pop() else {
                    diags.push(Diagnostic::new("IMD-102", no, "⟦KONIEC⟧ bez otwartego ⟦JEŻELI⟧ w linii blokowej"));
                    block_roles[idx] = Some(BlockRole::Unmatched);
                    skip_blank = true;
                    continue;
                };
                if indent != top.indent {
                    diags.push(indent_warning(d, indent, &top));
                }
                block_roles[idx] = Some(BlockRole::Close(id));
                top.close = Some(idx);
                push_node(&mut root, &mut stack, Node::Block(top));
            }
            Keyword::Row => unreachable!("row directives are never block lines"),
        }
        skip_blank = true;
    }
    while let Some((id, top)) = stack.pop() {
        diags.push(Diagnostic::new(
            "IMD-101",
            top.open + 1,
            "⟦JEŻELI⟧ bez ⟦KONIEC⟧: blok nie jest zamknięty do końca pliku",
        ));
        for &l in &branch_lines[id] {
            block_roles[l] = Some(BlockRole::Unmatched);
        }
        push_node(&mut root, &mut stack, Node::Block(top));
    }

    // ---- the blank line after every block line, and TABELA targets -------
    for idx in 0..n {
        let Some(d) = block_directive(&scan.kinds[idx]) else { continue };
        let no = idx + 1;
        if idx + 1 < n && !scan.lines[idx + 1].is_blank() {
            diags.push(Diagnostic::new(
                "IMD-105",
                no,
                format!(
                    "po linii {} musi stać pusta linia (bez niej następna linia wejdzie do cytatu, a kompilator usunie nie tę linię)",
                    d.raw
                ),
            ));
        }
        if d.keyword == Keyword::Table {
            check_table_target(scan, idx, &mut diags);
        }
    }

    // ---- inline groups and row directives --------------------------------
    let mut token_roles: Vec<Vec<TokenRole>> = Vec::with_capacity(n);
    for idx in 0..n {
        let LineKind::Text { tokens } = &scan.kinds[idx] else {
            token_roles.push(Vec::new());
            continue;
        };
        let (roles, groups, rows) = inline_roles(scan, idx, tokens, &mut diags);
        stats.inline += groups;
        stats.rows += rows;
        token_roles.push(roles);
    }

    // ---- expressions ------------------------------------------------------
    for (idx, kind) in scan.kinds.iter().enumerate() {
        let mut check = |d: &Directive| {
            if let Some(e) = &d.expr {
                expr::check(e, vocab, idx + 1, &mut diags);
            }
        };
        match kind {
            LineKind::Block { directive, .. } => check(directive),
            LineKind::Text { tokens } => tokens.iter().for_each(|t| {
                if let Token::Directive(d) = t {
                    check(d)
                }
            }),
            _ => {}
        }
    }

    // ---- units of stubbed blocks -----------------------------------------
    check_units(scan, &root, &mut diags);

    // ---- leftovers of the provisional HTML markers ------------------------
    for range in scan.layout.comments() {
        let body = scan.text[range.clone()].trim_start().trim_start_matches("<!--").trim_start();
        let marker = ["@if", "@elif", "@else", "@end", "@row", "@table"].into_iter().find(|m| {
            body.strip_prefix(m).is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphanumeric()))
        });
        if let Some(m) = marker {
            diags.push(Diagnostic::new(
                "IMD-701",
                scan.line_of(range.start),
                format!("znacznik prowizoryczny „<!-- {m} … -->” sprzed ITA-MD 1.0; przepisz go na dyrektywę (rozdział o migracji)"),
            ));
        }
    }

    Analysis { block_roles, token_roles, tree: root, stats, diagnostics: diags }
}

fn push_node(root: &mut Vec<Node>, stack: &mut [(usize, BlockNode)], node: Node) {
    match stack.last_mut() {
        Some((_, top)) => top.target().push(node),
        None => root.push(node),
    }
}

fn indent_warning(d: &Directive, indent: usize, top: &BlockNode) -> Diagnostic {
    Diagnostic::new(
        "IMD-106",
        d.line,
        format!(
            "{} ma wcięcie {indent}, a ⟦JEŻELI⟧ z linii {} ma wcięcie {}; linie jednego bloku stoją w tym samym miejscu struktury dokumentu",
            d.raw,
            top.open + 1,
            top.indent
        ),
    )
}

/// TABELA must be followed (after blank lines) by a table whose header
/// names tiers.
fn check_table_target(scan: &Scan, idx: usize, diags: &mut Vec<Diagnostic>) {
    let no = idx + 1;
    let next = (idx + 1..scan.lines.len()).find(|&j| !scan.lines[j].is_blank());
    let table = next.and_then(|j| {
        let l = &scan.lines[j];
        scan.layout.table_starting_in(l.start..l.start + l.text.len() + 1)
    });
    let Some(table) = table else {
        diags.push(Diagnostic::new("IMD-304", no, "po ⟦TABELA · kolumny: poziom⟧ (i pustych liniach) musi zaczynać się tabela"));
        return;
    };
    // The header row is the table's first line.
    let header_end = scan.text[table.start..].find('\n').map(|i| table.start + i).unwrap_or(table.end);
    if split_cells(&scan.text[table.start..header_end]).iter().all(|c| tier_in(c).is_none()) {
        diags.push(Diagnostic::new(
            "IMD-307",
            no,
            "nagłówek tabeli pod ⟦TABELA⟧ nie zawiera nazw Poziomów T1–T5; kompilator nie usunie żadnej kolumny",
        ));
    }
}

/// Cells of a GFM table line, as the reference generator splits them.
pub(crate) fn split_cells(line: &str) -> Vec<&str> {
    let mut s = line.trim();
    if let Some(r) = s.strip_prefix('|') {
        s = r;
    }
    if let Some(r) = s.strip_suffix('|') {
        s = r;
    }
    s.split('|').map(str::trim).collect()
}

/// The tier a table header cell names (`T1`–`T5` as a separate word).
pub(crate) fn tier_in(cell: &str) -> Option<&'static str> {
    let chars: Vec<(usize, char)> = cell.char_indices().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    for (k, &(_, c)) in chars.iter().enumerate() {
        if c != 'T' {
            continue;
        }
        let Some(&(_, d)) = chars.get(k + 1) else { continue };
        if !('1'..='5').contains(&d) {
            continue;
        }
        let before_ok = k == 0 || !word(chars[k - 1].1);
        let after_ok = chars.get(k + 2).is_none_or(|&(_, a)| !word(a));
        if before_ok && after_ok {
            return Some(expr::TIERS[(d as u8 - b'1') as usize]);
        }
    }
    None
}

/// Roles of a text line's tokens; returns (roles, inline groups, rows).
fn inline_roles(scan: &Scan, idx: usize, tokens: &[Token], diags: &mut Vec<Diagnostic>) -> (Vec<TokenRole>, usize, usize) {
    let no = idx + 1;
    let line = &scan.lines[idx];
    let cell = |t: &Token| scan.layout.cell_index(line.start + t.span().start);
    let mut roles = vec![TokenRole::Broken; tokens.len()];
    // Open groups: (token indices, else seen, cell of the opening token, broken).
    let mut stack: Vec<(Vec<usize>, bool, Option<usize>, bool)> = Vec::new();
    let mut finished: Vec<(Vec<usize>, bool)> = Vec::new();
    let mut rows = 0;
    for (ti, tok) in tokens.iter().enumerate() {
        let Token::Directive(d) = tok else { continue };
        match d.keyword {
            Keyword::Row => {
                rows += 1;
                if rows > 1 {
                    diags.push(Diagnostic::new("IMD-302", no, "w wierszu tabeli może stać tylko jedna dyrektywa ⟦WIERSZ⟧"));
                    continue;
                }
                if let Some(why) = row_misplacement(scan, idx, tok) {
                    diags.push(Diagnostic::new("IMD-301", no, format!("{}: {why}", d.raw)));
                    continue;
                }
                roles[ti] = TokenRole::Row;
            }
            Keyword::Table => {
                diags.push(Diagnostic::new(
                    "IMD-303",
                    no,
                    "TABELA stoi w osobnej linii blokowej przed tabelą: „> 🔀 ⟦TABELA · kolumny: poziom⟧”",
                ));
            }
            Keyword::If => stack.push((vec![ti], false, cell(tok), false)),
            Keyword::ElseIf | Keyword::Else => {
                let Some(top) = stack.last_mut() else {
                    diags.push(Diagnostic::new("IMD-203", no, format!("{} w linii bez wcześniejszego ⟦JEŻELI⟧ w tej linii", d.raw)));
                    continue;
                };
                if top.1 {
                    diags.push(Diagnostic::new("IMD-204", no, format!("{} po ⟦W PRZECIWNYM RAZIE⟧; gałąź „w przeciwnym razie” jest ostatnia", d.raw)));
                    top.3 = true;
                    continue;
                }
                if cell(tok) != top.2 {
                    diags.push(Diagnostic::new("IMD-205", no, format!("{} leży w innej komórce tabeli niż jego ⟦JEŻELI⟧", d.raw)));
                    top.3 = true;
                }
                top.1 |= d.keyword == Keyword::Else;
                top.0.push(ti);
            }
            Keyword::End => {
                let Some(mut top) = stack.pop() else {
                    diags.push(Diagnostic::new("IMD-202", no, "⟦KONIEC⟧ w linii bez wcześniejszego ⟦JEŻELI⟧ w tej linii"));
                    continue;
                };
                if cell(tok) != top.2 {
                    diags.push(Diagnostic::new("IMD-205", no, "⟦KONIEC⟧ leży w innej komórce tabeli niż jego ⟦JEŻELI⟧"));
                    top.3 = true;
                }
                top.0.push(ti);
                finished.push((top.0, top.3));
            }
        }
    }
    for (members, ..) in &stack {
        let d = match &tokens[members[0]] {
            Token::Directive(d) => d,
            Token::Bad { .. } => unreachable!(),
        };
        diags.push(Diagnostic::new("IMD-201", no, format!("{} bez ⟦KONIEC⟧ w tej samej linii", d.raw)));
    }
    let groups = finished.len();
    for (members, broken) in finished {
        if broken {
            continue;
        }
        let last = members.len() - 1;
        for (k, &ti) in members.iter().enumerate() {
            roles[ti] = match k {
                0 => TokenRole::Open,
                _ if k == last => TokenRole::Close,
                _ => TokenRole::Branch,
            };
        }
    }
    (roles, groups, rows)
}

/// Why a WIERSZ token is not at the start of a table row's first body cell
/// (`None` when it is).
fn row_misplacement(scan: &Scan, idx: usize, tok: &Token) -> Option<&'static str> {
    let line = &scan.lines[idx];
    let start = tok.span().start;
    let Some(ci) = scan.layout.cell_index(line.start + start) else {
        return Some("WIERSZ stoi wyłącznie w wierszu tabeli GFM");
    };
    let cell = scan.layout.cell(ci);
    if cell.header {
        return Some("WIERSZ nie może stać w wierszu nagłówka tabeli");
    }
    if !cell.first {
        return Some("WIERSZ stoi w pierwszej komórce wiersza");
    }
    let prefix = line.text[..start].trim();
    if prefix != "|" {
        return Some("WIERSZ stoi na samym początku pierwszej komórki, a wiersz zaczyna się od „|”");
    }
    None
}

/// The text of a line with its directives removed (as the reference
/// generator's `MARKER.sub("", line)`).
pub(crate) fn without_directives(scan: &Scan, idx: usize) -> String {
    let text = scan.lines[idx].text;
    let spans: Vec<std::ops::Range<usize>> = match &scan.kinds[idx] {
        LineKind::Block { directive, .. } => vec![directive.span.clone()],
        LineKind::Text { tokens } => tokens
            .iter()
            .filter_map(|t| match t {
                Token::Directive(d) => Some(d.span.clone()),
                Token::Bad { .. } => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for s in spans {
        out.push_str(&text[pos..s.start]);
        pos = s.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// `^(#{1,6})\s+(.*)$`: heading level and text.
pub(crate) fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some((level, rest.trim_start()))
}

/// `^(\s*)(\d+[a-z]?\.|[a-z]\.|\d+\))\s+(.*)$`: indentation, designation,
/// and the rest of a numbered or lettered item.
pub(crate) fn item(line: &str) -> Option<(&str, &str, &str)> {
    let body = line.trim_start();
    let indent = &line[..line.len() - body.len()];
    let b = body.as_bytes();
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    let des_len = if digits > 0 {
        match b.get(digits) {
            Some(b'.') => digits + 1,
            Some(b')') => digits + 1,
            Some(c) if c.is_ascii_lowercase() && b.get(digits + 1) == Some(&b'.') => digits + 2,
            _ => return None,
        }
    } else if b.first().is_some_and(u8::is_ascii_lowercase) && b.get(1) == Some(&b'.') {
        2
    } else {
        return None;
    };
    let rest = &body[des_len..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some((indent, &body[..des_len], rest.trim_start()))
}

/// The first non-blank line of a branch (descending into a leading nested
/// block's first branch), without directives.
pub(crate) fn head_line(scan: &Scan, nodes: &[Node]) -> String {
    for node in nodes {
        match node {
            Node::Block(b) => return head_line(scan, &b.branches[0].1),
            Node::Line(i) if !scan.lines[*i].is_blank() => return without_directives(scan, *i),
            Node::Line(_) => {}
        }
    }
    String::new()
}

fn check_units(scan: &Scan, nodes: &[Node], diags: &mut Vec<Diagnostic>) {
    for node in nodes {
        let Node::Block(b) = node else { continue };
        let all = b.branches.iter().chain(b.otherwise.as_ref());
        for (_, children) in all.clone() {
            check_units(scan, children, diags);
        }
        // An unclosed block is already an error; its extent is meaningless.
        if b.stub == Stub::None || b.close.is_none() {
            continue;
        }
        let head = head_line(scan, &b.branches[0].1);
        if heading(&head).is_some() {
            continue;
        }
        let no = b.open + 1;
        if item(&head).is_none() {
            diags.push(Diagnostic::new(
                "IMD-602",
                no,
                format!(
                    "pierwsza linia bloku („{}”) nie jest nagłówkiem ani punktem numerowanym, więc zaślepka „{}” nie ma oznaczenia; użyj „zaślepka: brak” albo zacznij blok od jednostki",
                    head.chars().take(60).collect::<String>(),
                    b.stub.word()
                ),
            ));
        }
        for (line, children) in all {
            let units = children
                .iter()
                .filter(|c| match c {
                    Node::Line(i) => {
                        let t = scan.lines[*i].text;
                        heading(t).is_some() || item(t).is_some_and(|(ind, ..)| ind.len() == b.indent)
                    }
                    Node::Block(_) => false,
                })
                .count();
            if units > 1 {
                diags.push(Diagnostic::new(
                    "IMD-601",
                    line + 1,
                    format!(
                        "gałąź obejmuje {units} jednostki na poziomie bloku; zaślepka zastąpiłaby je jednym oznaczeniem i numeracja by zniknęła — podziel blok na jednostki"
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::lex::scan;
    use super::*;

    fn codes(text: &str) -> Vec<&'static str> {
        let s = scan(text);
        let a = analyze(&s, None);
        let mut c: Vec<&str> = s.diagnostics.iter().chain(&a.diagnostics).map(|d| d.code).collect();
        c.sort();
        c
    }

    #[test]
    fn test_heading_and_item_matchers() {
        assert_eq!(heading("## E10. Tryb CITO"), Some((2, "E10. Tryb CITO")));
        assert_eq!(heading("###   § 1a. X"), Some((3, "§ 1a. X")));
        assert_eq!(heading("####### x"), None);
        assert_eq!(heading("#x"), None);
        assert_eq!(heading(" ## x"), None);
        assert_eq!(item("6. **Tytuł.** treść"), Some(("", "6.", "**Tytuł.** treść")));
        assert_eq!(item("   a. tekst"), Some(("   ", "a.", "tekst")));
        assert_eq!(item("1a. tekst"), Some(("", "1a.", "tekst")));
        assert_eq!(item("1) tekst"), Some(("", "1)", "tekst")));
        assert_eq!(item("12. x"), Some(("", "12.", "x")));
        assert_eq!(item("A. x"), None);
        assert_eq!(item("6.x"), None);
        assert_eq!(item("ab. x"), None);
        assert_eq!(item("- x"), None);
    }

    #[test]
    fn test_tier_in_header_cells() {
        assert_eq!(tier_in("**T1 Essential**"), Some("T1"));
        assert_eq!(tier_in("T5"), Some("T5"));
        assert_eq!(tier_in("T5A"), None);
        assert_eq!(tier_in("XT3"), None);
        assert_eq!(tier_in("Poziom"), None);
        assert_eq!(split_cells("| a | b |"), ["a", "b"]);
        assert_eq!(split_cells("a | b"), ["a", "b"]);
    }

    #[test]
    fn test_well_formed_blocks_and_tree() {
        let text = "> 🔀 ⟦JEŻELI a⟧\n\n1. x\n> 🔀 ⟦ALBO JEŻELI b⟧\n\n1. y\n> 🔀 ⟦W PRZECIWNYM RAZIE⟧\n\n1. z\n> 🔀 ⟦KONIEC⟧\n\ntail\n";
        let s = scan(text);
        let a = analyze(&s, None);
        assert!(s.diagnostics.is_empty() && a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        assert_eq!(a.block_roles[0], Some(BlockRole::Open(0)));
        assert_eq!(a.block_roles[3], Some(BlockRole::Branch(0)));
        assert_eq!(a.block_roles[6], Some(BlockRole::Branch(0)));
        assert_eq!(a.block_roles[9], Some(BlockRole::Close(0)));
        assert_eq!(a.stats.blocks, 1);
        let Node::Block(b) = &a.tree[0] else { panic!() };
        assert_eq!(b.branches.len(), 2);
        assert!(b.otherwise.is_some());
        // The blank line after each directive line is consumed, as in the reference.
        assert!(matches!(b.branches[0].1[..], [Node::Line(2)]));
        assert!(matches!(a.tree[1..], [Node::Line(11)]));
    }

    #[test]
    fn test_block_structure_errors() {
        assert_eq!(codes("> 🔀 ⟦JEŻELI a⟧\n\ntext\n"), ["IMD-101"]);
        assert_eq!(codes("text\n> 🔀 ⟦KONIEC⟧\n"), ["IMD-102"]);
        assert_eq!(codes("> 🔀 ⟦ALBO JEŻELI a⟧\n"), ["IMD-103"]);
        assert_eq!(codes("> 🔀 ⟦W PRZECIWNYM RAZIE⟧\n"), ["IMD-103"]);
        assert_eq!(
            codes("> 🔀 ⟦JEŻELI a⟧\n\n1. x\n> 🔀 ⟦W PRZECIWNYM RAZIE⟧\n\n1. y\n> 🔀 ⟦ALBO JEŻELI b⟧\n\ntekst\n> 🔀 ⟦KONIEC⟧\n"),
            ["IMD-104"]
        );
        assert_eq!(codes("> 🔀 ⟦JEŻELI a⟧\n1. x\n> 🔀 ⟦KONIEC⟧\n"), ["IMD-105"]);
        assert_eq!(codes("> 🔀 ⟦JEŻELI a⟧\n\n1. x\n   > 🔀 ⟦KONIEC⟧\n"), ["IMD-106"]);
        // Directive lines inside code never count.
        assert!(codes("```\n> 🔀 ⟦KONIEC⟧\n```\n").is_empty());
    }

    #[test]
    fn test_inline_groups() {
        let text = "a ⟦JEŻELI x⟧b⟦ALBO JEŻELI y⟧c⟦W PRZECIWNYM RAZIE⟧d⟦KONIEC⟧ e (⟦JEŻELI z⟧f⟦KONIEC⟧)\n";
        let s = scan(text);
        let a = analyze(&s, None);
        assert!(a.diagnostics.is_empty());
        use TokenRole::*;
        assert_eq!(a.token_roles[0], [Open, Branch, Branch, Close, Open, Close]);
        assert_eq!(a.stats.inline, 2);
        // Nested groups.
        let s = scan("⟦JEŻELI a⟧x (⟦JEŻELI b⟧y⟦W PRZECIWNYM RAZIE⟧z⟦KONIEC⟧)⟦KONIEC⟧\n");
        let a = analyze(&s, None);
        assert!(a.diagnostics.is_empty());
        assert_eq!(a.token_roles[0], [Open, Open, Branch, Close, Close]);
    }

    #[test]
    fn test_inline_errors() {
        assert_eq!(codes("a ⟦JEŻELI x⟧ b\n"), ["IMD-201"]);
        assert_eq!(codes("a ⟦KONIEC⟧ b\n"), ["IMD-202"]);
        assert_eq!(codes("a ⟦KONIEC⟧⟦JEŻELI x⟧ b\n"), ["IMD-201", "IMD-202"]);
        assert_eq!(codes("a ⟦W PRZECIWNYM RAZIE⟧ b\n"), ["IMD-203"]);
        assert_eq!(codes("⟦JEŻELI x⟧a⟦W PRZECIWNYM RAZIE⟧b⟦ALBO JEŻELI y⟧c⟦KONIEC⟧\n"), ["IMD-204"]);
        assert_eq!(codes("| ⟦JEŻELI x⟧a | b⟦KONIEC⟧ |\n|---|---|\n| c | d |\n"), ["IMD-205"]);
        assert_eq!(codes("| h | h2 |\n|---|---|\n| ⟦JEŻELI x⟧a | b⟦KONIEC⟧ |\n"), ["IMD-205"]);
        assert!(codes("| h | h2 |\n|---|---|\n| ⟦JEŻELI x⟧a⟦KONIEC⟧ | b |\n").is_empty());
    }

    #[test]
    fn test_row_and_table_placement() {
        let ok = "| P | T1 | T2 |\n|---|---|---|\n| ⟦WIERSZ tier == T1⟧**T1** | x | y |\n";
        assert!(codes(ok).is_empty());
        assert_eq!(codes("⟦WIERSZ a⟧ text\n"), ["IMD-301"]);
        assert_eq!(codes("| ⟦WIERSZ a⟧h | h2 |\n|---|---|\n| c | d |\n"), ["IMD-301"]);
        assert_eq!(codes("| h | h2 |\n|---|---|\n| c | ⟦WIERSZ a⟧d |\n"), ["IMD-301"]);
        assert_eq!(codes("| h | h2 |\n|---|---|\n| c ⟦WIERSZ a⟧ | d |\n"), ["IMD-301"]);
        assert_eq!(codes("| h | h2 |\n|---|---|\n| ⟦WIERSZ a⟧⟦WIERSZ b⟧c | d |\n"), ["IMD-302"]);
        assert_eq!(codes("a ⟦TABELA · kolumny: poziom⟧ b\n"), ["IMD-303"]);
        let table = "> 🔀 ⟦TABELA · kolumny: poziom⟧\n\n| Poziom | T1 | T2 |\n|---|---|---|\n| x | 1 | 2 |\n";
        assert!(codes(table).is_empty());
        assert_eq!(codes("> 🔀 ⟦TABELA · kolumny: poziom⟧\n\ntext\n"), ["IMD-304"]);
        assert_eq!(codes("> 🔀 ⟦TABELA · kolumny: poziom⟧\n\n| a | b |\n|---|---|\n| 1 | 2 |\n"), ["IMD-307"]);
    }

    #[test]
    fn test_units_of_stubbed_blocks() {
        // One item: fine. Heading head: any number of items.
        assert!(codes("> 🔀 ⟦JEŻELI a⟧\n\n6. **X.** treść\n> 🔀 ⟦KONIEC⟧\n").is_empty());
        assert!(codes("> 🔀 ⟦JEŻELI a⟧\n\n## E5. X\n\n1. a\n2. b\n> 🔀 ⟦KONIEC⟧\n").is_empty());
        assert_eq!(codes("> 🔀 ⟦JEŻELI a⟧\n\n6. x\n7. y\n> 🔀 ⟦KONIEC⟧\n"), ["IMD-601"]);
        assert_eq!(codes("> 🔀 ⟦JEŻELI a⟧\n\nZwykły akapit.\n> 🔀 ⟦KONIEC⟧\n"), ["IMD-602"]);
        // `zaślepka: brak` needs no designation.
        assert!(codes("> 🔀 ⟦JEŻELI a · zaślepka: brak⟧\n\nZwykły akapit.\n> 🔀 ⟦KONIEC⟧\n").is_empty());
        // Items deeper than the block do not count as units of the block.
        assert!(codes("> 🔀 ⟦JEŻELI a⟧\n\n6. x\n\n   a. sub\n\n   b. sub\n> 🔀 ⟦KONIEC⟧\n").is_empty());
    }

    #[test]
    fn test_provisional_markers() {
        assert_eq!(codes("<!-- @if tier >= T4 -->\n\ntext\n\n<!-- @end -->\n"), ["IMD-701", "IMD-701"]);
        assert_eq!(codes("a <!-- @row api --> b\n"), ["IMD-701"]);
        assert!(codes("<!-- @ifx -->\n<!-- zwykły komentarz -->\n").is_empty());
        assert!(codes("```\n<!-- @if a -->\n```\n").is_empty());
    }
}
