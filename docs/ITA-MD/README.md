# ITA-MD

Standard dyrektyw warunkowych w źródłach Markdown: jeden kanoniczny tekst, z
którego kompilator wytwarza egzemplarz dla każdej konfiguracji.

| Plik | Zawartość |
|---|---|
| [podrecznik-ITA-MD.md](podrecznik-ITA-MD.md) | **Podręcznik standardu ITA-MD** — jedyne źródło wiedzy o standardzie: składnia, znaczenie, zachowanie narzędzi, katalog błędów, reguły autorskie, migracja, przykład |
| [CHANGELOG.md](CHANGELOG.md) | dziennik zmian standardu (ITA-MD MAJOR.MINOR), odrębny od wersji md-files |
| [przyklady/](przyklady/) | przykład z rozdziału 19: fragment Cennika, słownik faktów, dwa zestawy faktów |

Obecna wersja: **ITA-MD 1.0**.

Szybki start:

```bash
mdread przyklady/cennik-fragment.md                                  # źródło z oznaczonymi warunkami
mdread --itamd-profile przyklady/fakty-T4.json przyklady/cennik-fragment.md   # egzemplarz dla T4
md2html --itamd-profile przyklady/fakty-T3.json przyklady/cennik-fragment.md -o T3.html
md2docx --itamd-profile przyklady/fakty-T3.json przyklady/cennik-fragment.md -o T3.docx
```
