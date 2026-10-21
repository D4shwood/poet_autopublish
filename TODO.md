# Future ideas

- **Essays (deneme).** A prose collection: a subfolder like the old poems,
  laid out as justified paragraphs, with a "New essay" item in the menu.
- **Page preview (`p`), suspended.** A text proof of the pages inside the
  terminal. It didn't work well in practice, so it is off by default; the code
  is kept behind the `preview` feature (`cargo build --features preview`).
- **Export a poem as an image.** A PNG of a single poem page in the series style,
  ready for sharing: a 1080×1080 square (long poems become a carousel of several
  images) or a 1080×1920 story.
- **Writing reminder.** An optional desktop nudge after a few days without a new poem.

## Ports

- **Windows**, so friends can install it easily. In phases:
  1. ~~Builds and runs on Windows~~ (done): CI tests and builds it; files open
     with Explorer, Notepad is the fallback editor, file names are Windows-safe,
     and the app works without TeX (no PDFs yet).
  2. ~~An installer and a portable .zip~~ (done): a per-user installer with
     Start menu entries, the `poet` command and an uninstaller, plus a zip,
     both built by the release workflow.
  3. Optional background rebuild at sign-in, with a Task Scheduler task.
  4. PDFs: a TeX engine that needs no separate install (likely Tectonic,
     bundled), with the fonts embedded.
- **Android app.** A mobile app for writing and reading on the phone: the
  writing screen, drafts and signing, and the published PDFs. The poem parsing,
  week grouping and authors logic can be shared as a Rust library; compiling
  PDFs on the phone may not be practical, so it could show the PDFs built on
  the computer instead.

## Stage 2

- **Syncing between phone and computer.** Poems written on either side end up
  in the same poems folder, and the PDFs come back to the phone. To be decided:
  a synced folder (e.g. Syncthing) versus the app's own sync, and how to handle
  the same poem edited on both sides.

## Ideas to explore

Angles that use what only this app knows: when every poem was written, every
draft, the authors and publications, and the Turkish text itself.

- **Time fingerprint.** Each weekly volume's colophon carries a small clock
  dial of the hours (and ☾ nights) its poems were written.
- **Moon at signing.** The moon phase, computed offline, next to the signature
  time: `☾ dolunay · 23.40`.
- **This day last year.** The blank page shows what you wrote a year ago today.
- **Night book.** Every poem written past midnight gathered into its own
  anthology.
- **Booklet imposition.** A volume or an issue laid out for A4 double-sided,
  fold-and-staple printing at home.
- **Fanzine mode.** An issue built from several authors' poems, a section each,
  with cover and contents (for issues like Mürekkep #3).
- **Constraint of the day.** A writing constraint drawn from your own corpus
  (three words from an old poem, a meter), faint on the blank page.
- **Reading mode.** Full screen, one stanza at a time, for poetry nights.
- **Proof of authorship.** OpenTimestamps proof for each signed poem, kept in
  `.poet/`.
- **Dead lines.** Every line cut while drafting, gathered in its own space and
  sometimes offered back on the blank page.
- **Generative covers.** Each volume's cover pattern drawn from its poems
  (line lengths, syllables, hours), the same poems always giving the same face.
- **Image constellations.** Poems linked by shared images; jump between them,
  and a constellation map at the end of the book.
- **Chain poem (renga).** Written with friends through the shared repo, each
  link starting from the last image of the one before, names in the margin.
- **Image concordance.** An index page of the recurring words across poems.
