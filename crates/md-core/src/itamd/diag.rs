//! The ITA-MD error catalogue: every problem a tool can report has a stable
//! code (`IMD-nnn`), a severity, and a one-line Polish title. The handbook's
//! error chapter documents exactly this table (a test keeps them in sync).

use std::fmt;

/// How serious a diagnostic is. Errors make a source non-conforming and stop
/// a compiler; warnings point at style or likely mistakes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    /// The Polish word used in reports.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "błąd",
            Severity::Warning => "ostrzeżenie",
        }
    }
}

/// One entry of the error catalogue.
pub struct CatalogueEntry {
    pub code: &'static str,
    pub severity: Severity,
    pub title: &'static str,
}

const fn e(code: &'static str, title: &'static str) -> CatalogueEntry {
    CatalogueEntry { code, severity: Severity::Error, title }
}

const fn w(code: &'static str, title: &'static str) -> CatalogueEntry {
    CatalogueEntry { code, severity: Severity::Warning, title }
}

/// The complete error catalogue of ITA-MD 1.0, in code order.
pub const CATALOGUE: &[CatalogueEntry] = &[
    // 0xx: lexical form of a directive
    e("IMD-001", "Niezamknięta dyrektywa: brak nawiasu zamykającego w tej samej linii"),
    e("IMD-002", "Nieznane słowo kluczowe po nawiasie otwierającym"),
    e("IMD-003", "Samotny nawias zamykający"),
    e("IMD-004", "Nieprawidłowa linia dyrektywy blokowej"),
    w("IMD-005", "Niekanoniczny zapis dyrektywy"),
    e("IMD-006", "Nieznana opcja"),
    e("IMD-007", "Nieznana wartość opcji"),
    e("IMD-008", "Opcja niedozwolona w tym miejscu"),
    e("IMD-009", "Powtórzona opcja"),
    e("IMD-010", "Brak wyrażenia"),
    e("IMD-011", "Zbędne wyrażenie"),
    // 1xx: block directives
    e("IMD-101", "JEŻELI bez KONIEC (blok)"),
    e("IMD-102", "KONIEC bez JEŻELI (blok)"),
    e("IMD-103", "ALBO JEŻELI albo W PRZECIWNYM RAZIE poza blokiem"),
    e("IMD-104", "Gałąź po W PRZECIWNYM RAZIE (blok)"),
    e("IMD-105", "Brak pustej linii po dyrektywie blokowej"),
    w("IMD-106", "Różne wcięcie linii jednego bloku"),
    // 2xx: inline directives
    e("IMD-201", "JEŻELI w linii bez KONIEC w tej samej linii"),
    e("IMD-202", "KONIEC w linii bez JEŻELI"),
    e("IMD-203", "ALBO JEŻELI albo W PRZECIWNYM RAZIE w linii bez JEŻELI"),
    e("IMD-204", "Gałąź po W PRZECIWNYM RAZIE (w linii)"),
    e("IMD-205", "Dyrektywa w linii przekracza granicę komórki tabeli"),
    // 3xx: rows and tables
    e("IMD-301", "WIERSZ poza początkiem pierwszej komórki wiersza tabeli"),
    e("IMD-302", "Więcej niż jedna dyrektywa WIERSZ w wierszu"),
    e("IMD-303", "TABELA poza linią blokową"),
    e("IMD-304", "Po dyrektywie TABELA nie następuje tabela"),
    e("IMD-305", "TABELA bez opcji „kolumny: poziom”"),
    e("IMD-306", "WIERSZ w linii blokowej"),
    w("IMD-307", "Tabela pod dyrektywą TABELA nie ma kolumn Poziomów"),
    // 4xx: expressions
    e("IMD-401", "Błąd składni wyrażenia"),
    e("IMD-402", "Identyfikator spoza słownika faktów"),
    e("IMD-403", "Porównanie kolejności poza Poziomami"),
    w("IMD-404", "Identyfikator przypomina stałą spoza zakresu"),
    e("IMD-405", "Wartość spoza dziedziny faktu"),
    // 5xx: vocabulary and fact sets
    e("IMD-501", "Nieprawidłowy plik słownika faktów"),
    e("IMD-502", "Słownik dla innej wersji głównej standardu"),
    e("IMD-503", "Brak faktu w zestawie faktów"),
    e("IMD-504", "Fakt spoza słownika w zestawie faktów"),
    e("IMD-505", "Wartość faktu niezgodna z typem"),
    e("IMD-506", "Nieprawidłowy plik faktów"),
    // 6xx: compilation units and results
    e("IMD-601", "Blok z zaślepką obejmuje więcej niż jedną jednostkę"),
    e("IMD-602", "Blok z zaślepką bez oznaczenia jednostki"),
    e("IMD-603", "Pozostałość dyrektywy po kompilacji"),
    e("IMD-604", "Porównanie kolejności wartości niebędących Poziomami"),
    // 7xx: migration
    e("IMD-701", "Pozostałość znacznika prowizorycznego w komentarzu HTML"),
];

/// The catalogue entry for `code`. Panics on an unknown code: codes are
/// compile-time constants of this crate, so a miss is a programming error.
pub fn entry(code: &str) -> &'static CatalogueEntry {
    CATALOGUE
        .iter()
        .find(|e| e.code == code)
        .unwrap_or_else(|| panic!("unknown ITA-MD code {code}"))
}

/// A problem found in a source, a vocabulary, or a fact set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Catalogue code, e.g. `IMD-101`.
    pub code: &'static str,
    pub severity: Severity,
    /// 1-based source line; 0 when the problem concerns a whole file.
    pub line: usize,
    /// Polish explanation of this occurrence.
    pub message: String,
}

impl Diagnostic {
    /// A diagnostic with the catalogue severity of `code`.
    pub fn new(code: &'static str, line: usize, message: impl Into<String>) -> Diagnostic {
        let entry = entry(code);
        Diagnostic { code: entry.code, severity: entry.severity, line, message: message.into() }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "linia {}: ", self.line)?;
        }
        write!(f, "{} ({}): {}", self.code, self.severity.label(), self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalogue_codes_are_unique_and_sorted() {
        let codes: Vec<&str> = CATALOGUE.iter().map(|e| e.code).collect();
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(codes, sorted);
        for c in codes {
            assert!(c.starts_with("IMD-") && c.len() == 7, "{c}");
        }
    }

    #[test]
    fn test_diagnostic_takes_catalogue_severity_and_formats() {
        let d = Diagnostic::new("IMD-005", 7, "podwójna spacja");
        assert_eq!(d.severity, Severity::Warning);
        assert!(!d.is_error());
        assert_eq!(d.to_string(), "linia 7: IMD-005 (ostrzeżenie): podwójna spacja");
        let f = Diagnostic::new("IMD-501", 0, "zły JSON");
        assert_eq!(f.to_string(), "IMD-501 (błąd): zły JSON");
    }

    #[test]
    #[should_panic(expected = "unknown ITA-MD code")]
    fn test_unknown_code_panics() {
        let _ = entry("IMD-999");
    }
}
