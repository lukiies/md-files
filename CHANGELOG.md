# Changelog

All notable changes to md-files are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
