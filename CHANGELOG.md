# Changelog

All notable changes to md-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.4.0] - 2026-10-12

ITA-MD 1.0: conditional directives in Markdown sources. One canonical source
says in which configuration a block, a table row, or a fragment of a
sentence applies; the suite shows such sources, strips them with a warning,
or compiles them for one configuration. The standard is described by its
own handbook and versioned separately from the tools.

### Added
- **docs/ITA-MD** — *Podręcznik standardu ITA-MD* (Polish handbook, version
  1.0): the source → deliverable model, conformance levels (author, viewer,
  compiler), lexical rules and full grammar, every directive with correct and
  incorrect examples, nesting, GFM tables, expressions, the fact vocabulary
  file `itamd-slownik.json` and fact sets, compilation semantics with the
  exact stub wordings, the pass order, fail-closed behaviour, the error
  catalogue IMD-001…IMD-701, authoring rules, reading sources in mdread,
  migration from the provisional HTML-comment markers, a worked example from
  the contract templates, and a glossary. The standard has its own changelog
  (`docs/ITA-MD/CHANGELOG.md`, ITA-MD MAJOR.MINOR). The handbook's examples
  are executable tests.
- **md-core** — `itamd` module: code-aware lexer and checker with the full
  error catalogue (`check`), the viewer's rendering (`annotate`: framed
  branches, humanised condition chips, stub badges, row and table chips, a
  diagnostics panel, a show/hide toolbar; light and dark palettes, no
  external assets), `strip_directives`, the compiler (`compile`, with
  `project` and `cleanup` for generators that add their own steps),
  vocabulary and fact-set loading, and a small strict JSON reader. The
  compiler output is byte-identical to the contract generator's reference
  implementation (78 template × profile cases and 52 stored copy parts); an
  opt-in golden test (`ITAMD_BAU_DIR`) repeats the comparison.
- **mdread** — sources with directives are shown annotated with a
  "Warunki: pokaż / ukryj" switch (remembered in the config file) and a
  "Podgląd dla profilu…" button that compiles the source for a chosen fact
  set (stubs dimmed; `--itamd-profile FACTS` opens the preview directly).
  The smoke mode reports how many ITA-MD chips and dimmed stubs the page
  rendered.
- **md2html** — sources are rendered annotated by default (the same view as
  mdread); `--itamd-profile FACTS` compiles.
- **md2docx** — `--itamd-profile FACTS` compiles before converting.

### Changed
- **md2docx** and **md2html --strip-notes** strip ITA-MD directives (every
  branch kept) and warn "plik zawiera dyrektywy ITA-MD 1.0 — to źródło, nie
  egzemplarz; użyj kompilatora"; `md2docx --final` refuses a source that
  still holds directives.

### Fixed
- **md-core** — working notes are recognised by an emoji first character
  instead of "any character above U+00FF": quotations that open with a
  Polish letter or a quotation mark (`> „Cytat…”`, `> Łukasz …`) are no
  longer stripped from deliverables.
- **md-core** — note, comment, and directive stripping never touches code
  blocks or code spans, so documents that show notes or directives as
  examples keep them; blank-line collapsing no longer alters code blocks.

## [0.3.1] - 2026-08-12

### Fixed
- **md2docx** — user input is validated before pandoc is looked for:
  `--reference-doc` existence and the `--final` TBC guard now report their
  own errors on machines without pandoc instead of being masked by the
  "pandoc not found" install instructions. The stripped-notes temp file is
  also cleaned up when pandoc turns out to be missing.

## [0.3.0] - 2026-08-12

The reader remembers its window.

### Added
- **mdread** — window layout persistence: position, size, and maximized state
  are saved to the config file (`md-files/mdread.conf` under the user config
  directory, next to the zoom level) when the window closes and on every zoom
  change, then restored on the next start. Geometry is tracked only while the
  window is not maximized, so maximizing never overwrites the normal layout.
  Implausible values — Windows' minimized parking position, sub-200 px sizes,
  corrupted config lines — are rejected and fall back to the 1000×800 default,
  so a damaged config can never open the window broken or off-screen.

## [0.2.0] - 2026-08-10

The source/deliverable release: the `.md` file is the editable source, the
converted output is the compiled deliverable, and working notes never cross
that line.

### Added
- **md-core** — `strip_source_notes`: removes source-only annotations —
  emoji-blockquote notes (`> 🟢 **[DRAFTED]** …`, `> 🟠 **[EXPANSION
  NOTE]** …`, `> 📝 **[WORKING NOTE]** …`, `> 🖼️ **[FIGURE NOTE]** …`,
  `> 📋 **[SOURCE NOTE]** …`, continuation lines included), HTML comments
  (multi-line too), and legacy bare `[DRAFTED…]`/`[EXPANSION NOTE…]` lines.
  `[TABLE OF CONTENTS]`/`[TABLE OF FIGURES]`/`[LIST OF TABLES]` marker notes
  are stripped but reported to the caller. `count_tbc` counts word-bounded
  `TBC` placeholders (the convention's only sanctioned placeholder).
- **md2docx** — strips working notes before conversion by default;
  `--keep-notes` converts the source verbatim; `--final` refuses to convert
  while any `TBC` placeholder remains; a `> 🧭 **[TABLE OF CONTENTS]**`
  marker note turns into a pandoc table of contents.
- **md2html** — `--strip-notes` previews the deliverable (same stripping as
  md2docx); `--save` writes the `.html` next to the input file (built for the
  Explorer context menu).
- **mdread** — `MDREAD_SMOKE=1` smoke mode: exits 0 once the first page
  reports DOMContentLoaded through IPC (proves the protocol handler served
  the page and the WebView rendered it), 3 on timeout.
- **install-windows.ps1** — Explorer right-click entries for `.md`/
  `.markdown`: Open in mdread, Convert to HTML, Convert to Word; mdread now
  also claims the per-user extension default progid and gets a file icon.
- Integration test suites for all three binaries (`md2html` end-to-end CLI
  tests, `md2docx` CLI tests with pandoc-dependent cases skipped when pandoc
  is absent, `mdread` Windows render smoke test) and unit tests for mdread's
  URL/zoom helpers.
- `demo/` folder with manual-test documents.

### Fixed
- **mdread** on Windows: wry maps custom protocols to
  `https://mdfiles.localhost` only when the https scheme is explicitly
  enabled; without it every page failed with `ERR_CONNECTION_REFUSED`.
  The smoke test now guards this pipeline.

### Changed
- **md2docx** treats the `.md` as editable source: working notes are now
  stripped from the `.docx` by default (previous verbatim behavior is behind
  `--keep-notes`).

## [0.1.0] - 2026-08-05

Initial release: the MDView fork's renderer generalized into a cross-platform
tool suite.

### Added
- **md-core** — shared markdown-to-HTML engine ported from the MDView fork:
  GFM (tables, footnotes, strikethrough, task lists), mermaid diagram blocks,
  highlight.js code coloring, local-image resolution with serve-root widening
  (images outside the document folder render from disk, different-drive images
  inline as `data:` URIs), and a new `.md`-link rewrite hook for viewers.
  UTF-8 BOM at the start of a document no longer breaks the first heading.
- **mdread** — cross-platform GUI reader (wry + tao: WebView2 on Windows,
  WebKitGTK on Linux, WKWebView on macOS). Pages are served on demand through
  a custom `mdfiles` protocol, so there is no document size limit and links
  between .md files navigate with native Back/Forward history. Live reload
  (metadata polling, file never held open, scroll position preserved), zoom
  persisted to a config file, system dark-mode detection with `--dark`/
  `--light` override, embedded offline mermaid/highlight bundles, ESC to
  close, Ctrl+O open dialog.
- **md2html** — standalone converter (successor of MDView's `--html`/`--body`
  modes): CDN-asset pages by default, `--embed` for a fully self-contained
  offline file with bundles and local images inlined, `--dark`, `--title`,
  stdin input.
- **md2docx** — pandoc front-end for Word output with `--reference-doc`,
  `--toc`, automatic resource path, and pandoc discovery (PATH, the Windows
  per-user installer location, common Unix prefixes).
- Install scripts for Windows (`install\install-windows.ps1`: per-user bin,
  PATH, Open With registration) and Linux/macOS (`install/install-unix.sh`:
  `~/.local/bin` or `/usr/local/bin`, Linux desktop entry).
- GitHub Actions workflow building and testing on Windows, Linux, and macOS.
