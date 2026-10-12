//! A small, strict JSON reader (RFC 8259) for the suite's configuration
//! files: ITA-MD fact vocabularies and fact sets. Objects keep their key
//! order; duplicate keys are an error rather than "last one wins", so a
//! mistyped file never silently changes meaning.

use std::fmt;

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    /// Key/value pairs in file order (keys are unique).
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value under `key` when `self` is an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string value, if `self` is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// The object's pairs, if `self` is an object.
    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// A short Polish name of the value's type, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "wartość logiczna",
            Json::Number(_) => "liczba",
            Json::String(_) => "tekst",
            Json::Array(_) => "tablica",
            Json::Object(_) => "obiekt",
        }
    }
}

/// Where and why parsing failed (1-based line and column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "linia {}, kolumna {}: {}", self.line, self.column, self.message)
    }
}

/// Parse a complete JSON document (a leading UTF-8 BOM is allowed).
pub fn parse(text: &str) -> Result<Json, JsonError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Reader { text, pos: 0, depth: 0 };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.pos < text.len() {
        return Err(p.error("nadmiarowa treść po wartości JSON"));
    }
    Ok(value)
}

struct Reader<'a> {
    text: &'a str,
    pos: usize,
    depth: usize,
}

impl Reader<'_> {
    fn error(&self, message: &str) -> JsonError {
        let before = &self.text[..self.pos.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0) + 1;
        JsonError { line, column, message: message.to_string() }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, literal: &str) -> bool {
        if self.text[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, JsonError> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') if self.eat("true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("oczekiwano wartości JSON")),
            None => Err(self.error("nieoczekiwany koniec pliku")),
        }
    }

    fn enter(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(self.error("zbyt głębokie zagnieżdżenie"));
        }
        Ok(())
    }

    fn object(&mut self) -> Result<Json, JsonError> {
        self.enter()?;
        self.pos += 1; // '{'
        let mut pairs: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.eat("}") {
            self.depth -= 1;
            return Ok(Json::Object(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.error("oczekiwano klucza w cudzysłowie"));
            }
            let key_pos = self.pos;
            let key = self.string()?;
            if pairs.iter().any(|(k, _)| *k == key) {
                self.pos = key_pos;
                return Err(self.error(&format!("powtórzony klucz \"{key}\"")));
            }
            self.skip_ws();
            if !self.eat(":") {
                return Err(self.error("oczekiwano „:” po kluczu"));
            }
            self.skip_ws();
            let value = self.value()?;
            pairs.push((key, value));
            self.skip_ws();
            if self.eat(",") {
                continue;
            }
            if self.eat("}") {
                self.depth -= 1;
                return Ok(Json::Object(pairs));
            }
            return Err(self.error("oczekiwano „,” albo „}”"));
        }
    }

    fn array(&mut self) -> Result<Json, JsonError> {
        self.enter()?;
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat("]") {
            self.depth -= 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            if self.eat(",") {
                continue;
            }
            if self.eat("]") {
                self.depth -= 1;
                return Ok(Json::Array(items));
            }
            return Err(self.error("oczekiwano „,” albo „]”"));
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let rest = &self.text[self.pos..];
            let Some(c) = rest.chars().next() else {
                return Err(self.error("niezakończony tekst"));
            };
            match c {
                '"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                '\\' => {
                    self.pos += 1;
                    let Some(e) = self.peek() else {
                        return Err(self.error("niezakończona sekwencja ucieczki"));
                    };
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&first) {
                                if !self.eat("\\u") {
                                    return Err(self.error("niesparowany surogat UTF-16"));
                                }
                                let second = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&second) {
                                    return Err(self.error("niesparowany surogat UTF-16"));
                                }
                                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&first) {
                                return Err(self.error("niesparowany surogat UTF-16"));
                            } else {
                                first
                            };
                            match char::from_u32(code) {
                                Some(ch) => out.push(ch),
                                None => return Err(self.error("nieprawidłowy znak \\u")),
                            }
                        }
                        _ => return Err(self.error("nieznana sekwencja ucieczki")),
                    }
                }
                c if (c as u32) < 0x20 => {
                    return Err(self.error("znak sterujący w tekście (użyj sekwencji ucieczki)"))
                }
                c => {
                    out.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self.text.get(self.pos..self.pos + 4).unwrap_or("");
        if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(self.error("oczekiwano czterech cyfr szesnastkowych"));
        }
        self.pos += 4;
        Ok(u32::from_str_radix(digits, 16).unwrap())
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        let mut i = self.pos;
        if bytes.get(i) == Some(&b'-') {
            i += 1;
        }
        match bytes.get(i) {
            Some(b'0') => i += 1,
            Some(b'1'..=b'9') => {
                while matches!(bytes.get(i), Some(b'0'..=b'9')) {
                    i += 1;
                }
            }
            _ => return Err(self.error("nieprawidłowa liczba")),
        }
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            if !matches!(bytes.get(i), Some(b'0'..=b'9')) {
                return Err(self.error("nieprawidłowa liczba"));
            }
            while matches!(bytes.get(i), Some(b'0'..=b'9')) {
                i += 1;
            }
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            i += 1;
            if matches!(bytes.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            if !matches!(bytes.get(i), Some(b'0'..=b'9')) {
                return Err(self.error("nieprawidłowa liczba"));
            }
            while matches!(bytes.get(i), Some(b'0'..=b'9')) {
                i += 1;
            }
        }
        self.pos = i;
        self.text[start..i]
            .parse::<f64>()
            .map(Json::Number)
            .map_err(|_| self.error("nieprawidłowa liczba"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_nested_document_in_order() {
        let v = parse(r#"{"b": [1, -2.5e1, true, null], "a": {"x": "y"}}"#).unwrap();
        let pairs = v.as_object().unwrap();
        assert_eq!(pairs[0].0, "b");
        assert_eq!(pairs[1].0, "a");
        assert_eq!(
            v.get("b"),
            Some(&Json::Array(vec![
                Json::Number(1.0),
                Json::Number(-25.0),
                Json::Bool(true),
                Json::Null
            ]))
        );
        assert_eq!(v.get("a").unwrap().get("x").unwrap().as_str(), Some("y"));
    }

    #[test]
    fn test_string_escapes_and_unicode() {
        let v = parse(r#""a\"b\\c\n\u017b\ud83d\udd00 Łódź""#).unwrap();
        assert_eq!(v, Json::String("a\"b\\c\nŻ🔀 Łódź".to_string()));
    }

    #[test]
    fn test_bom_and_whitespace_allowed() {
        assert_eq!(parse("\u{feff}  {}  \n").unwrap(), Json::Object(vec![]));
    }

    #[test]
    fn test_errors_carry_position() {
        let e = parse("{\n  \"a\": 1,\n  \"a\": 2\n}").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("powtórzony klucz"), "{e}");
        assert!(parse("{\"a\": 1,}").is_err()); // trailing comma
        assert!(parse("[1 2]").is_err());
        assert!(parse("tru").is_err());
        assert!(parse("01").is_err());
        assert!(parse("\"\\ud800\"").is_err());
        assert!(parse("{} x").is_err());
        assert!(parse("\"a\nb\"").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn test_type_names_are_polish() {
        assert_eq!(Json::Bool(true).type_name(), "wartość logiczna");
        assert_eq!(Json::Object(vec![]).type_name(), "obiekt");
    }
}
