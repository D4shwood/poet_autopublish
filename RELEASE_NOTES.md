First public beta. Write poems in the terminal; they become weekly PDF volumes.

**What's in it**

- A writing screen with drafts (`#taslak`) and signing with Ctrl+G.
- Signatures read for author, date and time. Approximate dates (`~08.10.2026`),
  a night cutoff (poems before 05:00 count for the day before), pen names, and
  the publication after the date (`Mürekkep #2`, `Örnek Yayın : Almanak`).
- One PDF per week of your series, one cumulative book, and a book for each
  collection (subfolder), rebuilt only when their poems change.
- Authors and pen names: only your own poems are published, others are tracked.
- Screens for signatures, publications, stats, search and settings.
- A first-run setup and an optional background service that rebuilds on change.

**Install**

Needs XeLaTeX (`texlive-xetex texlive-latexrecommended texlive-latexextra`)
and the Liberation fonts. Unpack the archive and put `poetautopublish` in
`~/.local/bin`; the first run walks you through the rest. Linux x86_64 only.

**Beta notes**

The poem files are never changed by the build, but keep a backup of your poems
folder. Please report problems in Issues.
