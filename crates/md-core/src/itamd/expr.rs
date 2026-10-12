//! ITA-MD expressions: the condition after `JEŻELI`, `ALBO JEŻELI` and
//! `WIERSZ`.
//!
//! ```text
//! wyrażenie    = alternatywa
//! alternatywa  = koniunkcja { "or" koniunkcja }
//! koniunkcja   = negacja { "and" negacja }
//! negacja      = "not" negacja | porównanie
//! porównanie   = składnik [ operator składnik | "in" lista | "not" "in" lista ]
//! składnik     = fakt | stała | "True" | "False" | "(" wyrażenie ")"
//! lista        = "[" stała { "," stała } "]"
//! ```
//!
//! Evaluation follows the reference generator: a constant stands for its own
//! name, a bare fact means "true" (logical fact) or "not empty" (text fact),
//! equality never errors, and ordering is defined only between the tiers
//! T1 < T2 < T3 < T4 < T5 (by that explicit order, never by string order).

use std::collections::BTreeMap;
use std::fmt;

use super::diag::Diagnostic;
use super::vocab::{FactType, Vocabulary};

/// The service tiers, in their contractual order (the only ordered values).
pub const TIERS: [&str; 5] = ["T1", "T2", "T3", "T4", "T5"];

/// True if `name` is one of the ITA-MD 1.0 constants: T1–T5, I1–I4, L1–L4,
/// P1–P5, M0–M3, T5A–T5C.
pub fn is_constant(name: &str) -> bool {
    matches!(
        name.as_bytes(),
        [b'T', b'1'..=b'5']
            | [b'I', b'1'..=b'4']
            | [b'L', b'1'..=b'4']
            | [b'P', b'1'..=b'5']
            | [b'M', b'0'..=b'3']
            | [b'T', b'5', b'A'..=b'C']
    )
}

/// True if `name` has the shape of a constant (a family letter, digits, an
/// optional capital) without being one — `T6`, `L0`, `T5D`.
pub fn looks_like_constant(name: &str) -> bool {
    let b = name.as_bytes();
    if b.len() < 2 || !matches!(b[0], b'T' | b'I' | b'L' | b'P' | b'M') {
        return false;
    }
    let digits = b[1..].iter().take_while(|c| c.is_ascii_digit()).count();
    let rest = &b[1 + digits..];
    digits > 0 && (rest.is_empty() || (rest.len() == 1 && rest[0].is_ascii_uppercase())) && !is_constant(name)
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    fn symbol(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        }
    }

    fn human(self) -> &'static str {
        match self {
            CmpOp::Eq => "=",
            CmpOp::Ne => "≠",
            CmpOp::Lt => "<",
            CmpOp::Le => "≤",
            CmpOp::Gt => ">",
            CmpOp::Ge => "≥",
        }
    }

    fn is_ordering(self) -> bool {
        matches!(self, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge)
    }
}

/// A parsed expression. Parentheses written by the author are kept as
/// [`Expr::Group`], so printing reproduces the source structure exactly.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Or(Vec<Expr>),
    And(Vec<Expr>),
    Not(Box<Expr>),
    Compare(Box<Expr>, CmpOp, Box<Expr>),
    In { item: Box<Expr>, list: Vec<String>, negated: bool },
    Fact(String),
    Const(String),
    Bool(bool),
    Group(Box<Expr>),
}

/// A syntax error with its 1-based character column within the expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    pub column: usize,
    pub message: String,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "kolumna {}: {}", self.column, self.message)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    And,
    Or,
    Not,
    In,
    True,
    False,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Op(CmpOp),
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("„{s}”"),
        Tok::And => "„and”".into(),
        Tok::Or => "„or”".into(),
        Tok::Not => "„not”".into(),
        Tok::In => "„in”".into(),
        Tok::True => "„True”".into(),
        Tok::False => "„False”".into(),
        Tok::LParen => "„(”".into(),
        Tok::RParen => "„)”".into(),
        Tok::LBracket => "„[”".into(),
        Tok::RBracket => "„]”".into(),
        Tok::Comma => "„,”".into(),
        Tok::Op(op) => format!("„{}”", op.symbol()),
    }
}

fn tokenize(text: &str) -> Result<Vec<(Tok, usize)>, ExprError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let err = |column: usize, message: String| Err(ExprError { column: column + 1, message });
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let start = i;
        let tok = match c {
            c if c.is_whitespace() => {
                i += 1;
                continue;
            }
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ',' => Tok::Comma,
            '=' if next == Some('=') => {
                i += 1;
                Tok::Op(CmpOp::Eq)
            }
            '=' => return err(start, "pojedynczy znak „=”; porównanie zapisuje się „==”".into()),
            '!' if next == Some('=') => {
                i += 1;
                Tok::Op(CmpOp::Ne)
            }
            '!' => return err(start, "znak „!”; zaprzeczenie zapisuje się „not”, nierówność „!=”".into()),
            '<' if next == Some('=') => {
                i += 1;
                Tok::Op(CmpOp::Le)
            }
            '<' => Tok::Op(CmpOp::Lt),
            '>' if next == Some('=') => {
                i += 1;
                Tok::Op(CmpOp::Ge)
            }
            '>' => Tok::Op(CmpOp::Gt),
            '&' => return err(start, "znak „&”; koniunkcję zapisuje się „and”".into()),
            '|' => return err(start, "znak „|”; alternatywę zapisuje się „or”".into()),
            '"' | '\'' | '„' | '”' | '“' => {
                return err(start, "cudzysłów; stałe i fakty zapisuje się bez cudzysłowu".into())
            }
            '≥' | '≤' | '≠' => {
                return err(start, format!("znak „{c}”; w wyrażeniu używa się „>=”, „<=”, „!=”"))
            }
            c if c.is_ascii_digit() => {
                return err(start, "liczba; w wyrażeniach ITA-MD nie ma liczb (stałe to np. T4, L2)".into())
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_alphanumeric() {
                    return err(
                        j,
                        format!(
                            "znak „{}” w identyfikatorze; identyfikatory składają się z liter łacińskich bez znaków diakrytycznych, cyfr i „_”",
                            chars[j]
                        ),
                    );
                }
                let word: String = chars[i..j].iter().collect();
                i = j - 1;
                match word.as_str() {
                    "and" => Tok::And,
                    "or" => Tok::Or,
                    "not" => Tok::Not,
                    "in" => Tok::In,
                    "True" => Tok::True,
                    "False" => Tok::False,
                    _ => Tok::Ident(word),
                }
            }
            c if c.is_alphabetic() => {
                return err(
                    start,
                    format!("znak „{c}”; identyfikatory składają się z liter łacińskich bez znaków diakrytycznych, cyfr i „_”"),
                )
            }
            c => return err(start, format!("niedozwolony znak „{c}”")),
        };
        out.push((tok, start));
        i += 1;
    }
    Ok(out)
}

/// Parse an expression. An empty text is an error (directives that take no
/// expression never call this).
pub fn parse(text: &str) -> Result<Expr, ExprError> {
    let toks = tokenize(text)?;
    let width = text.chars().count();
    if toks.is_empty() {
        return Err(ExprError { column: 1, message: "puste wyrażenie".into() });
    }
    let mut p = ExprParser { toks, pos: 0, width };
    let e = p.or()?;
    if let Some((t, col)) = p.toks.get(p.pos) {
        let hint = match t {
            Tok::Op(_) => " (porównania nie łączą się w łańcuch; użyj „and”)",
            Tok::Ident(w) if ["AND", "OR", "NOT", "IN"].contains(&w.as_str()) => {
                " (operatory piszemy małymi literami)"
            }
            Tok::Ident(w) if ["i", "oraz", "lub", "albo", "nie"].contains(&w.as_str()) => {
                " (operatory: „and”, „or”, „not”)"
            }
            _ => "",
        };
        return Err(ExprError {
            column: col + 1,
            message: format!("nadmiarowy symbol {} po wyrażeniu{hint}", describe(t)),
        });
    }
    Ok(e)
}

struct ExprParser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    width: usize,
}

impl ExprParser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn column(&self) -> usize {
        self.toks.get(self.pos).map(|(_, c)| c + 1).unwrap_or(self.width + 1)
    }

    fn fail<T>(&self, message: String) -> Result<T, ExprError> {
        Err(ExprError { column: self.column(), message })
    }

    fn found(&self) -> String {
        match self.peek() {
            Some(t) => describe(t),
            None => "koniec wyrażenia".into(),
        }
    }

    fn or(&mut self) -> Result<Expr, ExprError> {
        let mut items = vec![self.and()?];
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            items.push(self.and()?);
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Expr::Or(items) })
    }

    fn and(&mut self) -> Result<Expr, ExprError> {
        let mut items = vec![self.not()?];
        while self.peek() == Some(&Tok::And) {
            self.pos += 1;
            items.push(self.not()?);
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Expr::And(items) })
    }

    fn not(&mut self) -> Result<Expr, ExprError> {
        if self.peek() == Some(&Tok::Not) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, ExprError> {
        let left = self.term()?;
        match self.peek() {
            Some(Tok::Op(op)) => {
                let op = *op;
                self.pos += 1;
                let right = self.term()?;
                Ok(Expr::Compare(Box::new(left), op, Box::new(right)))
            }
            Some(Tok::In) => {
                self.pos += 1;
                let list = self.list()?;
                Ok(Expr::In { item: Box::new(left), list, negated: false })
            }
            Some(Tok::Not) => {
                self.pos += 1;
                if self.peek() != Some(&Tok::In) {
                    return self.fail(format!("po „not” w tym miejscu oczekiwano „in”, jest {}", self.found()));
                }
                self.pos += 1;
                let list = self.list()?;
                Ok(Expr::In { item: Box::new(left), list, negated: true })
            }
            _ => Ok(left),
        }
    }

    fn term(&mut self) -> Result<Expr, ExprError> {
        let Some(tok) = self.peek().cloned() else {
            return self.fail("niepełne wyrażenie: oczekiwano faktu, stałej albo „(”".into());
        };
        match tok {
            Tok::Ident(name) => {
                self.pos += 1;
                Ok(if is_constant(&name) { Expr::Const(name) } else { Expr::Fact(name) })
            }
            Tok::True => {
                self.pos += 1;
                Ok(Expr::Bool(true))
            }
            Tok::False => {
                self.pos += 1;
                Ok(Expr::Bool(false))
            }
            Tok::LParen => {
                self.pos += 1;
                let inner = self.or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return self.fail(format!("oczekiwano „)”, jest {}", self.found()));
                }
                self.pos += 1;
                Ok(Expr::Group(Box::new(inner)))
            }
            Tok::LBracket => self.fail("lista może stać tylko po „in” albo „not in”".into()),
            _ => self.fail(format!("oczekiwano faktu, stałej albo „(”, jest {}", self.found())),
        }
    }

    fn list(&mut self) -> Result<Vec<String>, ExprError> {
        if self.peek() != Some(&Tok::LBracket) {
            return self.fail(format!("po „in” oczekiwano listy „[…]”, jest {}", self.found()));
        }
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            match self.peek().cloned() {
                Some(Tok::Ident(name)) if is_constant(&name) => {
                    self.pos += 1;
                    items.push(name);
                }
                Some(Tok::Ident(name)) => {
                    return self.fail(format!("„{name}” nie jest stałą; lista zawiera wyłącznie stałe"))
                }
                _ => return self.fail(format!("oczekiwano stałej na liście, jest {}", self.found())),
            }
            match self.peek() {
                Some(Tok::Comma) => self.pos += 1,
                Some(Tok::RBracket) => {
                    self.pos += 1;
                    return Ok(items);
                }
                _ => return self.fail(format!("oczekiwano „,” albo „]”, jest {}", self.found())),
            }
        }
    }
}

impl Expr {
    /// The canonical spelling: single spaces around operators, `, ` in
    /// lists, the author's parentheses kept.
    pub fn canonical(&self) -> String {
        self.print(&|n: &str| n.to_string(), false)
    }

    /// A reader-friendly Polish rendering: `and` → „i”, `or` → „lub”,
    /// `not` → „nie”, `>=` → „≥”, `==` → „=”, `in` → „w”; facts are shown by
    /// their vocabulary label when `label` knows one.
    pub fn humanize(&self, label: &dyn Fn(&str) -> Option<String>) -> String {
        self.print(&|n: &str| label(n).unwrap_or_else(|| n.to_string()), true)
    }

    fn print(&self, name: &dyn Fn(&str) -> String, human: bool) -> String {
        let join = |items: &[Expr], sep: &str| {
            items.iter().map(|e| e.print(name, human)).collect::<Vec<_>>().join(sep)
        };
        match self {
            Expr::Or(items) => join(items, if human { " lub " } else { " or " }),
            Expr::And(items) => join(items, if human { " i " } else { " and " }),
            Expr::Not(inner) => format!("{} {}", if human { "nie" } else { "not" }, inner.print(name, human)),
            Expr::Compare(l, op, r) => format!(
                "{} {} {}",
                l.print(name, human),
                if human { op.human() } else { op.symbol() },
                r.print(name, human)
            ),
            Expr::In { item, list, negated } => {
                let word = match (human, negated) {
                    (false, false) => "in",
                    (false, true) => "not in",
                    (true, false) => "w",
                    (true, true) => "nie w",
                };
                if human {
                    format!("{} {word} {{{}}}", item.print(name, human), list.join(", "))
                } else {
                    format!("{} {word} [{}]", item.print(name, human), list.join(", "))
                }
            }
            Expr::Fact(n) => name(n),
            Expr::Const(c) => c.clone(),
            Expr::Bool(b) => match (human, b) {
                (false, true) => "True".into(),
                (false, false) => "False".into(),
                (true, true) => "prawda".into(),
                (true, false) => "fałsz".into(),
            },
            Expr::Group(inner) => format!("({})", inner.print(name, human)),
        }
    }

    /// Every fact name the expression reads, in order of appearance.
    pub fn facts(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_facts(&mut out);
        out
    }

    fn collect_facts<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Expr::Or(items) | Expr::And(items) => items.iter().for_each(|e| e.collect_facts(out)),
            Expr::Not(inner) | Expr::Group(inner) => inner.collect_facts(out),
            Expr::Compare(l, _, r) => {
                l.collect_facts(out);
                r.collect_facts(out);
            }
            Expr::In { item, .. } => item.collect_facts(out),
            Expr::Fact(n) => out.push(n),
            Expr::Const(_) | Expr::Bool(_) => {}
        }
    }
}

/// The value of a fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    /// A text fact; the empty text means "not set".
    Text(String),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Text(s) => !s.is_empty(),
        }
    }
}

/// A fact set: the facts of one configuration, by name.
pub type Facts = BTreeMap<String, Value>;

/// Why an expression could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// The expression reads a fact the fact set does not have (IMD-503).
    MissingFact(String),
    /// An ordering comparison between values that are not both tiers
    /// (IMD-604).
    Ordering(String, String),
}

impl EvalError {
    /// This error as a catalogue diagnostic at `line`.
    pub fn diagnostic(&self, line: usize, expr_text: &str) -> Diagnostic {
        match self {
            EvalError::MissingFact(name) => Diagnostic::new(
                "IMD-503",
                line,
                format!("wyrażenie „{expr_text}” czyta fakt „{name}”, którego nie ma w zestawie faktów"),
            ),
            EvalError::Ordering(a, b) => Diagnostic::new(
                "IMD-604",
                line,
                format!(
                    "wyrażenie „{expr_text}” porównuje kolejność wartości „{a}” i „{b}”; kolejność jest określona tylko dla Poziomów T1–T5"
                ),
            ),
        }
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Text(s) => s.clone(),
    }
}

impl Expr {
    /// Evaluate against `facts`. Like the reference generator, every operand
    /// of `and`/`or` is evaluated (no short-circuit), so a missing fact is
    /// reported even where it would not change the result.
    pub fn eval(&self, facts: &Facts) -> Result<bool, EvalError> {
        Ok(self.value(facts)?.truthy())
    }

    fn value(&self, facts: &Facts) -> Result<Value, EvalError> {
        Ok(match self {
            Expr::Or(items) => {
                let vals = items.iter().map(|e| e.value(facts)).collect::<Result<Vec<_>, _>>()?;
                Value::Bool(vals.iter().any(Value::truthy))
            }
            Expr::And(items) => {
                let vals = items.iter().map(|e| e.value(facts)).collect::<Result<Vec<_>, _>>()?;
                Value::Bool(vals.iter().all(Value::truthy))
            }
            Expr::Not(inner) => Value::Bool(!inner.value(facts)?.truthy()),
            Expr::Compare(l, op, r) => {
                let (a, b) = (l.value(facts)?, r.value(facts)?);
                let result = if op.is_ordering() {
                    let rank = |v: &Value| match v {
                        Value::Text(s) => TIERS.iter().position(|t| t == s),
                        Value::Bool(_) => None,
                    };
                    let (Some(x), Some(y)) = (rank(&a), rank(&b)) else {
                        return Err(EvalError::Ordering(show(&a), show(&b)));
                    };
                    match op {
                        CmpOp::Lt => x < y,
                        CmpOp::Le => x <= y,
                        CmpOp::Gt => x > y,
                        _ => x >= y,
                    }
                } else {
                    (a == b) == (*op == CmpOp::Eq)
                };
                Value::Bool(result)
            }
            Expr::In { item, list, negated } => {
                let found = match item.value(facts)? {
                    Value::Text(s) => list.contains(&s),
                    Value::Bool(_) => false,
                };
                Value::Bool(found != *negated)
            }
            Expr::Fact(name) => facts.get(name).cloned().ok_or_else(|| EvalError::MissingFact(name.clone()))?,
            Expr::Const(c) => Value::Text(c.clone()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Group(inner) => inner.value(facts)?,
        })
    }
}

/// Static checks of one expression: unknown identifiers (with a
/// vocabulary), ordering outside tiers, constants outside a fact's domain,
/// and identifiers that look like out-of-range constants.
pub(crate) fn check(expr: &Expr, vocab: Option<&Vocabulary>, line: usize, out: &mut Vec<Diagnostic>) {
    for name in expr.facts() {
        match vocab {
            Some(v) if v.get(name).is_none() => {
                let hint = if looks_like_constant(name) {
                    " (wygląda na stałą spoza zakresu standardu)"
                } else {
                    ""
                };
                out.push(Diagnostic::new(
                    "IMD-402",
                    line,
                    format!("„{name}” nie jest faktem słownika „{}”{hint}", v.display_name()),
                ));
            }
            None if looks_like_constant(name) => out.push(Diagnostic::new(
                "IMD-404",
                line,
                format!("„{name}” wygląda na stałą, ale nie należy do stałych ITA-MD 1.0; zostanie potraktowany jako fakt"),
            )),
            _ => {}
        }
    }
    check_types(expr, vocab, line, out);
}

/// The type of a term, as far as it can be known statically.
enum Kind<'a> {
    Tier,
    Logical,
    Enum,
    Const(&'a str),
    Unknown,
}

fn kind_of<'a>(e: &'a Expr, vocab: Option<&'a Vocabulary>) -> Kind<'a> {
    match e {
        Expr::Const(c) => Kind::Const(c),
        Expr::Bool(_) => Kind::Logical,
        Expr::Fact(name) => match vocab.and_then(|v| v.get(name)).map(|d| &d.kind) {
            Some(FactType::Tier) => Kind::Tier,
            Some(FactType::Logical) => Kind::Logical,
            Some(FactType::Enum(_)) => Kind::Enum,
            None => Kind::Unknown,
        },
        Expr::Group(inner) => kind_of(inner, vocab),
        Expr::Or(_) | Expr::And(_) | Expr::Not(_) | Expr::Compare(..) | Expr::In { .. } => Kind::Logical,
    }
}

fn check_types(expr: &Expr, vocab: Option<&Vocabulary>, line: usize, out: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Or(items) | Expr::And(items) => items.iter().for_each(|e| check_types(e, vocab, line, out)),
        Expr::Not(inner) | Expr::Group(inner) => check_types(inner, vocab, line, out),
        Expr::Compare(l, op, r) => {
            check_types(l, vocab, line, out);
            check_types(r, vocab, line, out);
            let text = expr.canonical();
            if op.is_ordering() {
                let why: Vec<String> = [l, r]
                    .into_iter()
                    .filter_map(|side| match kind_of(side, vocab) {
                        Kind::Tier | Kind::Unknown => None,
                        Kind::Const(c) if TIERS.contains(&c) => None,
                        Kind::Const(c) => Some(format!("stała „{c}” nie jest Poziomem")),
                        Kind::Logical => Some(format!("„{}” jest wartością logiczną", side.canonical())),
                        Kind::Enum => Some(format!("fakt „{}” nie jest Poziomem", side.canonical())),
                    })
                    .collect();
                if !why.is_empty() {
                    out.push(Diagnostic::new(
                        "IMD-403",
                        line,
                        format!(
                            "„{text}”: {}; porównania <, <=, >, >= są dozwolone tylko między Poziomami T1–T5",
                            why.join(", ")
                        ),
                    ));
                }
            } else {
                for (fact, other) in [(l, r), (r, l)] {
                    if let (Expr::Fact(name), Kind::Const(c)) = (fact.as_ref(), kind_of(other, vocab)) {
                        domain_check(name, &[c], vocab, &text, line, out);
                    }
                }
            }
        }
        Expr::In { item, list, .. } => {
            if let Expr::Fact(name) = item.as_ref() {
                let consts: Vec<&str> = list.iter().map(String::as_str).collect();
                domain_check(name, &consts, vocab, &expr.canonical(), line, out);
            }
        }
        Expr::Fact(_) | Expr::Const(_) | Expr::Bool(_) => {}
    }
}

fn domain_check(name: &str, consts: &[&str], vocab: Option<&Vocabulary>, text: &str, line: usize, out: &mut Vec<Diagnostic>) {
    let Some(def) = vocab.and_then(|v| v.get(name)) else { return };
    let domain: Vec<&str> = match &def.kind {
        FactType::Tier => TIERS.to_vec(),
        FactType::Enum(values) => values.iter().map(String::as_str).collect(),
        FactType::Logical => Vec::new(),
    };
    for c in consts {
        if !domain.contains(c) {
            let why = if matches!(def.kind, FactType::Logical) {
                format!("fakt „{name}” jest logiczny i nigdy nie jest równy stałej „{c}”")
            } else {
                format!("stała „{c}” nie należy do dziedziny faktu „{name}” ({})", domain.join(", "))
            };
            out.push(Diagnostic::new("IMD-405", line, format!("„{text}”: {why}; warunek nie może być spełniony")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(pairs: &[(&str, Value)]) -> Facts {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn t(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    #[test]
    fn test_constants() {
        for c in ["T1", "T5", "I1", "I4", "L1", "L4", "P1", "P5", "M0", "M3", "T5A", "T5B", "T5C"] {
            assert!(is_constant(c), "{c}");
        }
        for c in ["T0", "T6", "I0", "I5", "L5", "P0", "P6", "M4", "T5D", "T4A", "tier", "t1"] {
            assert!(!is_constant(c), "{c}");
        }
        assert!(looks_like_constant("T6") && looks_like_constant("L0") && looks_like_constant("T5D"));
        assert!(!looks_like_constant("T4") && !looks_like_constant("tier") && !looks_like_constant("Tx"));
    }

    #[test]
    fn test_parse_precedence_and_canonical_roundtrip() {
        for src in [
            "tier >= T4",
            "ef and not ef_dedicated",
            "infra_m1 or infra_m3 or ef",
            "holidays and standby24",
            "sla_intensity in [I1, I2, I3]",
            "sla_intensity not in [I4]",
            "(ef or api) and tier == T3",
            "not tier == T1",
            "True",
        ] {
            assert_eq!(parse(src).unwrap().canonical(), src);
        }
        // `and` binds tighter than `or`.
        assert_eq!(
            parse("a or b and c").unwrap(),
            Expr::Or(vec![
                Expr::Fact("a".into()),
                Expr::And(vec![Expr::Fact("b".into()), Expr::Fact("c".into())])
            ])
        );
        // `not` covers the whole comparison.
        assert!(matches!(parse("not tier == T1").unwrap(), Expr::Not(inner) if matches!(*inner, Expr::Compare(..))));
        assert_eq!(parse("tier>=T4").unwrap().canonical(), "tier >= T4");
        assert_eq!(parse("x in [I1,I2]").unwrap().canonical(), "x in [I1, I2]");
    }

    #[test]
    fn test_parse_errors_explain() {
        let cases = [
            ("", "puste"),
            ("tier = T4", "„==”"),
            ("a && b", "„and”"),
            ("a || b", "„or”"),
            ("!a", "„not”"),
            ("tier ≥ T4", "„>=”"),
            ("poziom_usług", "diakrytycznych"),
            ("T1 < tier < T4", "łańcuch"),
            ("a AND b", "małymi literami"),
            ("a i b", "„and”"),
            ("x in [I1, tier]", "nie jest stałą"),
            ("x in I1", "listy"),
            ("x not y", "oczekiwano „in”"),
            ("(a or b", "„)”"),
            ("tier == 4", "liczb"),
            ("tier == \"T4\"", "cudzysłowu"),
            ("a and", "niepełne"),
            ("[I1]", "tylko po"),
        ];
        for (src, needle) in cases {
            let err = parse(src).unwrap_err();
            assert!(err.message.contains(needle), "{src:?} -> {err}");
        }
        assert_eq!(parse("a = b").unwrap_err().column, 3);
    }

    #[test]
    fn test_humanize_in_polish() {
        let label = |n: &str| (n == "tier").then(|| "Poziom".to_string());
        assert_eq!(parse("tier >= T4").unwrap().humanize(&label), "Poziom ≥ T4");
        assert_eq!(parse("ef and not ef_dedicated").unwrap().humanize(&label), "ef i nie ef_dedicated");
        assert_eq!(parse("a or b").unwrap().humanize(&label), "a lub b");
        assert_eq!(parse("tier == T4").unwrap().humanize(&label), "Poziom = T4");
        assert_eq!(parse("tier != T4").unwrap().humanize(&label), "Poziom ≠ T4");
        assert_eq!(parse("tier <= T2").unwrap().humanize(&label), "Poziom ≤ T2");
        assert_eq!(parse("x in [I1, I2]").unwrap().humanize(&label), "x w {I1, I2}");
        assert_eq!(parse("x not in [I1]").unwrap().humanize(&label), "x nie w {I1}");
        assert_eq!(parse("(a or b) and True").unwrap().humanize(&label), "(a lub b) i prawda");
    }

    #[test]
    fn test_eval_matches_reference_semantics() {
        let f = facts(&[
            ("tier", t("T3")),
            ("ef", Value::Bool(true)),
            ("api", Value::Bool(false)),
            ("reservation", t("")),
            ("sla_intensity", t("I2")),
        ]);
        let ev = |s: &str| parse(s).unwrap().eval(&f);
        assert_eq!(ev("tier >= T3"), Ok(true));
        assert_eq!(ev("tier >= T4"), Ok(false));
        assert_eq!(ev("tier < T4"), Ok(true));
        assert_eq!(ev("tier == T3"), Ok(true));
        assert_eq!(ev("tier != T3"), Ok(false));
        assert_eq!(ev("ef and not api"), Ok(true));
        assert_eq!(ev("api or reservation"), Ok(false)); // empty text is false
        assert_eq!(ev("sla_intensity"), Ok(true)); // non-empty text is true
        assert_eq!(ev("sla_intensity in [I1, I2, I3]"), Ok(true));
        assert_eq!(ev("sla_intensity not in [I2, I3]"), Ok(false));
        assert_eq!(ev("ef in [I1]"), Ok(false)); // a logical value is never in a list
        assert_eq!(ev("ef == T1"), Ok(false)); // equality never errors
        assert_eq!(ev("(ef or api) and tier == T3"), Ok(true));
        assert_eq!(ev("True and not False"), Ok(true));
        // Tier order is explicit, not string order: T10 would sort before T2.
        assert_eq!(ev("tier > T2"), Ok(true));
    }

    #[test]
    fn test_eval_errors() {
        let f = facts(&[("tier", t("T3")), ("reservation", t("L2")), ("ef", Value::Bool(true))]);
        assert_eq!(parse("missing or ef").unwrap().eval(&f), Err(EvalError::MissingFact("missing".into())));
        // No short-circuit: the missing fact is reported even after a true operand.
        assert_eq!(parse("ef or missing").unwrap().eval(&f), Err(EvalError::MissingFact("missing".into())));
        assert_eq!(
            parse("reservation >= L1").unwrap().eval(&f),
            Err(EvalError::Ordering("L2".into(), "L1".into()))
        );
        assert_eq!(parse("ef > T1").unwrap().eval(&f), Err(EvalError::Ordering("True".into(), "T1".into())));
        let d = EvalError::MissingFact("x".into()).diagnostic(4, "x");
        assert_eq!((d.code, d.line), ("IMD-503", 4));
        assert_eq!(EvalError::Ordering("a".into(), "b".into()).diagnostic(1, "a > b").code, "IMD-604");
    }

    fn vocab() -> Vocabulary {
        Vocabulary::parse(
            r#"{"standard": "ITA-MD 1.0", "fakty": {
                "tier": {"typ": "poziom", "etykieta": "Poziom"},
                "ef": {"typ": "logiczny"},
                "reservation": {"typ": "wyliczeniowy", "wartości": ["L1", "L2", "L3", "L4"]}
            }}"#,
        )
        .unwrap()
    }

    fn codes(src: &str, v: Option<&Vocabulary>) -> Vec<&'static str> {
        let mut out = Vec::new();
        check(&parse(src).unwrap(), v, 1, &mut out);
        out.iter().map(|d| d.code).collect()
    }

    #[test]
    fn test_static_checks_with_vocabulary() {
        let v = vocab();
        assert!(codes("tier >= T4 and ef", Some(&v)).is_empty());
        assert_eq!(codes("standby24", Some(&v)), ["IMD-402"]);
        assert_eq!(codes("reservation >= L2", Some(&v)), ["IMD-403"]); // one per comparison
        assert_eq!(codes("ef > T1", Some(&v)), ["IMD-403"]);
        assert_eq!(codes("reservation == I1", Some(&v)), ["IMD-405"]);
        assert_eq!(codes("reservation in [L1, I2]", Some(&v)), ["IMD-405"]);
        assert_eq!(codes("ef == T1", Some(&v)), ["IMD-405"]);
        assert_eq!(codes("tier == T5A", Some(&v)), ["IMD-405"]);
        assert!(codes("tier in [T4, T5]", Some(&v)).is_empty());
    }

    #[test]
    fn test_static_checks_without_vocabulary() {
        assert!(codes("anything and other", None).is_empty());
        assert_eq!(codes("tier == T6", None), ["IMD-404"]);
        assert_eq!(codes("x >= L2", None), ["IMD-403"]);
        assert!(codes("x >= T2", None).is_empty());
    }
}
