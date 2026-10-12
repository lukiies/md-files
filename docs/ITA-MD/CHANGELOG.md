# Dziennik zmian standardu ITA-MD

Zmiany **standardu** ITA-MD (składni i znaczenia dyrektyw). Numeracja: ITA-MD
MAJOR.MINOR, opisana w rozdziale 2 [podręcznika](podrecznik-ITA-MD.md). Ten
dziennik jest odrębny od dziennika zmian narzędzi md-files
([../../CHANGELOG.md](../../CHANGELOG.md)): wersja narzędzi i wersja standardu
zmieniają się niezależnie.

Zasady wpisów: MAJOR zawiera instrukcję migracji źródeł; MINOR wymienia nowe
słowa kluczowe, opcje i możliwości; poprawki redakcyjne podręcznika bez zmiany
znaczenia wpisuje się z datą pod bieżącą wersją.

## [1.0] - 2026-10-12

Pierwsza wersja standardu.

### Dodano
- Dyrektywy w nawiasach `⟦ ⟧` (U+27E6, U+27E7) ze słowami kluczowymi
  `JEŻELI`, `ALBO JEŻELI`, `W PRZECIWNYM RAZIE`, `KONIEC`, `WIERSZ`, `TABELA`.
- Cztery rodzaje dyrektyw: blokowa (linia notatki roboczej `> 🔀 ⟦…⟧`, po której
  stoi pusta linia), w linii, wiersza (na początku pierwszej komórki wiersza
  tabeli GFM) i tabeli (`⟦TABELA · kolumny: poziom⟧`).
- Opcje `zaślepka: numer|tytuł|brak` i `kolumny: poziom`, oddzielane kropką
  środkową `·`.
- Wyrażenia: `or`, `and`, `not`, porównania `==`, `!=`, `<`, `<=`, `>`, `>=`
  (kolejność tylko między Poziomami T1 < T2 < T3 < T4 < T5), `in` i `not in`
  z listą stałych; stałe T1–T5, I1–I4, L1–L4, P1–P5, M0–M3, T5A–T5C.
- Znaczenie kompilacji: pierwsza prawdziwa gałąź, gałąź `W PRZECIWNYM RAZIE`
  albo zaślepka o dokładnym brzmieniu „Nie dotyczy niniejszej Umowy.” /
  „nie dotyczy niniejszej Umowy”; kolejność przebiegów bloki → wiersze →
  tabele Poziomu → w linii → porządkowanie; zasada fail closed.
- Słownik faktów projektu w pliku `itamd-slownik.json` (typy `logiczny`,
  `poziom`, `wyliczeniowy`; etykiety i opisy dla przeglądarek) i zestaw faktów
  w JSON.
- Katalog błędów IMD-001…IMD-701.
- Poziomy zgodności: autor, przeglądarka (z konwerterem bez kompilacji),
  kompilator.

### Zastąpiono
- Prowizoryczne znaczniki w komentarzach HTML (`<!-- @if … -->`, używane
  krótko 2026-10-12, bez numeru wersji). Migracja: rozdział 18 podręcznika;
  pozostałości wykrywa błąd IMD-701.
