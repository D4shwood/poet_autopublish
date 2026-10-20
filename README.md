# poet_autopublish

[Türkçe](README.tr.md)

Write poems in a quiet terminal page, and have them published automatically as
weekly PDF volumes of your own series (mine is called *Contra Noir*).

![A volume cover and a poem page](docs/pages.png)

- **Write**: a distraction-free writing screen with soft wrap, typewriter mode,
  autosave, drafts and one-key signing.
- **Collect**: poems are plain text files in one folder. The signature on the
  last line dates the poem and decides which week it belongs to.
- **Publish**: every Monday–Sunday week becomes a PDF volume (cover, one poem per
  page, colophon), plus one cumulative book with a table of contents. Only the
  volumes that changed are rebuilt, automatically, whenever a poem changes.

The program is `poetautopublish` (short: `poet`).

## Install

You need Rust and XeLaTeX with a few common packages. On Arch / CachyOS:

```sh
sudo pacman -S rust texlive-xetex texlive-latexrecommended texlive-latexextra ttf-liberation
cargo install --git https://github.com/D4shwood/poet_autopublish --root ~/.local
poetautopublish install
```

`install` sets up a systemd user service that rebuilds the PDFs whenever your
poems folder changes, adds the `poet` command to `~/.local/bin`, and adds a menu
entry (with a *New poem* action). `poetautopublish uninstall` removes all of it.

On first run a config is created at `~/.config/poetautopublish/config.toml`. Set
at least `author` (your name, exactly as you sign) and `poems_dir`. See
[config.example.toml](config.example.toml) for every option.

## Poem files

A poem is a UTF-8 file with no extension or a `.txt` extension:

```
Title of the poem

First line,
    indented lines keep their indentation,

a blank line starts a new stanza.

- Your Name. 13.10.2026 - 22.58
```

- **Line 1** is the title.
- **The last line is the signature.** Only signed poems are published. Variants
  like `Your Name 13/10/2026 22:58` or a date without a time also work.
- **Unsure of the day?** Put a `~` before the date: `- Your Name. ~08.10.2026`
  means "around 8 October". The poem goes into that week, after the poems
  with exact dates, and its footer shows only the week of the month
  (`October 2026 · week 2`).
- **Night writing:** a poem signed before 05:00 counts as the previous day
  (`night_cutoff`), so it stays in that night's week. The page still shows the
  real time.
- **Drafts:** put `#taslak` or `#draft` on the line after the title and the poem
  is never published, even when signed.
- **Epigraphs:** a file whose name starts with `_epigraf` holds a quote for a
  week's cover. Its last line is a date (`13.10.2026`) that picks the week.
- **Revisions:** if you edit a poem more than a day after signing it, its
  footer gets `rev. <date>`.

Hidden files, files starting with `_`, and backup files are ignored.

## Collections

Poems that don't belong to the weekly series, such as older work, go into a
**subfolder** of the poems folder, for example `~/poetry/Old Poems/`. Each
subfolder becomes its own book, `old_poems.pdf`. Its cover carries the folder
name and your name, and it has a table of contents. Poems are sorted
alphabetically, and only signed poems are published, as in the series.
Collections never appear in the weekly volumes or the series book.

You don't have to make the folders yourself. In the interface, *Collections*
lists every collection, including empty ones, and **+ New collection** asks
for a name and creates the folder. Press `n` with a collection selected to
write a new poem straight into it. A collection's book appears once it has its
first signed poem.

## Output

A week is named after its earliest poem ("5 Ekim Haftası" / "Week of 5
October") unless you rename it with `r` in the interface. Custom names are kept
in `~/.config/poetautopublish/weeks.toml`. They change the cover, the contents
and the PDF title, but not the file name.

For each week, `<series>_DD_MM_week.pdf` is named after the week's first poem,
for example `contra_noir_13_10_week.pdf`. The cumulative book is
`<series>_all.pdf`, with the newest week first. Set `mirror_dir` to also copy
every PDF into another folder, such as a synced one.

PDF labels and the interface are in Turkish when most of your poems are
Turkish, and in English otherwise (`lang = "tr" | "en" | "auto"`).

## Using it

```sh
poet            # the terminal interface
poet new        # straight to a blank page
poet build      # rebuild what changed (--force for everything)
poet watch      # what the service runs
```

In the interface:

| Key | |
|---|---|
| `n` | new poem |
| `r` | rename the selected week (empty = back to the default name) |
| `⏎` | open the selected poem in the writing screen |
| `/` | search all poems and their history |
| `h` | history of the selected poem |
| `o` | open the PDF (set `pdf_viewer` in the config to choose the program) |
| `b` / `f` | build / force a full rebuild |
| `D` | move the poem to the Trash |
| `s` | statistics |
| `w` | start, stop or install the background watcher |
| `l` | log |
| `e` | open the poem in `$EDITOR` |
| `q` / **End Noir** | quit; a running build is allowed to finish first |

In the writing screen: `Ctrl+S` saves (as a draft), `Ctrl+G` signs with the
current time and publishes, `Ctrl+T` toggles typewriter mode, `Ctrl+Z` undoes,
and `Esc` saves and goes back. Every save is kept in a local history
(`~/.local/share/poetautopublish/history`, the last 50 versions per poem).

When the background service isn't running, the interface watches the folder
itself while it's open.

## Customising the look

Copy [templates/template.tex](templates/template.tex) to
`~/.config/poetautopublish/template.tex` and edit it. Colours, fonts, margins
and the cover layout all live there, and no recompiling is needed.

Try it with the bundled example by setting `author = "Örnek Şair"` and running:

```sh
poet build --poems-dir examples/poems
```

## License

All rights reserved.
