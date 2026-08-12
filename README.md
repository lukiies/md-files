# md-files

One project for everything markdown: a fast cross-platform reader, an HTML
converter, and a docx converter. Written in Rust.

Successor of the [MDView fork](../MDView) — the reader carries over its
features (rich rendering, link navigation, live reload, large documents,
many image formats) but runs on Windows, Linux, and macOS instead of being
tied to WebView2 and Total Commander.

## The tools

| Tool | What it does |
|------|--------------|
| `mdread` | GUI markdown reader. Navigate between .md files by clicking links, with native Back/Forward history. Live reload, remembered window layout and zoom, dark mode, mermaid diagrams, syntax highlighting, local images of any size and count. |
| `md2html` | Markdown to HTML. Plain page (CDN assets) or fully self-contained offline file (`--embed`: bundles and images inlined). `--strip-notes` previews the deliverable (working notes removed). |
| `md2docx` | Markdown to Word. Front-end for pandoc with sane defaults (`--reference-doc`, `--toc`, resource path handling). Treats the .md as editable source: working notes are stripped from the .docx deliverable (`--keep-notes` to disable, `--final` to enforce a clean build). |

## Source vs deliverable: working notes

The suite treats a markdown file like source code and the converted document
as the compiled deliverable. Working notes live in the source, clearly
visible in any viewer, and never reach the converted output:

```markdown
> 🟢 **[DRAFTED]** this chapter is complete
> 🟠 **[EXPANSION NOTE]** add the Q3 numbers here later
> 📝 **[WORKING NOTE]** any other working discussion
> 🖼️ **[FIGURE NOTE]** figure provenance / rebuild instructions
> 📋 **[SOURCE NOTE]** build & status key info
```

Any blockquote whose first line opens with an emoji is a note — the whole
blockquote (continuation lines included) is stripped by `md2docx` (default)
and by `md2html --strip-notes`. `mdread` shows notes, as a viewer should.
HTML comments `<!-- … -->` are stripped too. Leave a blank line after a note
so the following prose is not absorbed into the blockquote.

Extras that come with the convention:

* `> 🧭 **[TABLE OF CONTENTS]**` — a marker note: stripped, but `md2docx`
  generates a pandoc table of contents in its place.
* `TBC` is the only sanctioned placeholder. It passes into interim builds;
  `md2docx --final` refuses to convert while any `TBC` remains outside notes.

All three share one engine (`md-core`): GitHub Flavored Markdown, footnotes,
task lists, ` ```mermaid ` diagram blocks, highlight.js code coloring, and
on-disk image resolution with the serve-root widening logic from the MDView
fork (images outside the document folder still render, however large).

## mdread architecture (why it is fast and has no size limits)

Every page is served on demand through a custom `mdfiles://` protocol.
A page URL encodes the absolute path of the markdown file it renders, so:

- nothing is pushed to the WebView as a string — the 2 MB `NavigateToString`
  limit that plagued large documents in MDView cannot exist here,
- a link to another .md file is a plain navigation: the WebView's own history
  drives mouse Back/Forward across files and folders,
- live reload is `location.reload()` — the handler re-reads the file from
  disk (only timestamps are polled, the file is never held open),
- images stream from disk with correct MIME types
  (png/jpg/gif/svg/webp/bmp/ico/avif/apng/tiff).

WebView backends: WebView2 (Windows), WebKitGTK (Linux), WKWebView (macOS).
The mermaid and highlight.js bundles are embedded in the binary — no internet
access needed.

### Keys

| Key | Action |
|-----|--------|
| ESC | Close |
| Ctrl+O | Open file |
| Ctrl+scroll, Ctrl+`+`/`-` | Zoom (persisted across sessions) |
| Ctrl+0 | Reset zoom |
| Mouse Back/Forward | History across linked .md files |

Window position, size, and maximized state are remembered on close and
restored on the next start, alongside the zoom level.

## Usage

```bash
# Reader
mdread README.md
mdread --dark notes.md

# HTML
md2html README.md -o README.html          # page using CDN assets
md2html --embed --dark notes.md -o n.html # single self-contained offline file
cat notes.md | md2html --body             # body fragment to stdout

# Word
md2docx report.md
md2docx report.md -o "Final Report.docx" --reference-doc styles.docx --toc
```

## Install

### Windows

```powershell
powershell -ExecutionPolicy Bypass -File install\install-windows.ps1
```

Builds, installs to `%LOCALAPPDATA%\Programs\md-files`, adds it to the user
PATH, registers mdread in the Open With list for `.md`/`.markdown`, and adds
Explorer right-click entries: Open in mdread, Convert to HTML (writes the
.html next to the file), Convert to Word (writes the .docx next to the file).
Uninstall with `-Uninstall`.

`md2docx` needs pandoc: `winget install --id JohnMacFarlane.Pandoc` (an
existing install in `%LOCALAPPDATA%\Pandoc` is found automatically).

### Linux

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev   # Debian/Ubuntu
sh install/install-unix.sh
```

Installs to `~/.local/bin` and adds an Open With desktop entry.
`md2docx` needs pandoc (`sudo apt install pandoc`).

### macOS

```bash
sh install/install-unix.sh    # installs to /usr/local/bin (may ask for sudo)
```

`md2docx` needs pandoc (`brew install pandoc`).

## Building from source

```bash
cargo build --release          # all three tools
cargo test --workspace         # engine tests
```

Requires Rust 1.85+ (edition 2021 workspace). CI builds all three platforms
(`.github/workflows/build.yml`).

## Project layout

```
crates/md-core/    shared markdown -> HTML engine (library)
crates/mdread/     GUI reader (wry + tao)
crates/md2html/    HTML converter CLI
crates/md2docx/    docx converter CLI (pandoc front-end)
assets/web/        embedded mermaid.js / highlight.js bundles
install/           per-platform install scripts
```

## Roadmap

- Heading anchors (`file.md#section` scrolls to the heading)
- Terminal (ANSI) output mode like MDView's
- In-page search (Ctrl+F)
- File tree sidebar for folder browsing

## License

MPL-2.0 (inherited from MDView, whose renderer this project ports).
Bundled: [mermaid](https://github.com/mermaid-js/mermaid) (MIT),
[highlight.js](https://github.com/highlightjs/highlight.js) (BSD-3-Clause).
