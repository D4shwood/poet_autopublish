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
- **Release binaries.** Attach a prebuilt Linux binary to tagged releases.

## Ports

- **Windows.** The same terminal app on Windows (Windows Terminal / PowerShell).
  The TUI already uses crossterm, which runs there; the Linux-only parts need
  Windows counterparts: the systemd background service (a scheduled task or a
  tray process instead), the `poet` symlink and the KDE menu entry (a Start menu
  shortcut), `xdg-open` (the default PDF handler), the freedesktop Trash, and a
  TeX distribution with XeLaTeX (MiKTeX or TeX Live).
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
