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
  2. An installer (.exe: Start menu shortcut, the `poet` command, uninstall
     entry) and a portable .zip, both built by CI on each release.
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
