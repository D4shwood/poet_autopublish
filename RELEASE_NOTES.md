Second beta: now on Windows too. Write poems in the terminal; they become weekly PDF volumes.

**New in this beta**

- **Windows.** `…-windows-setup.exe` installs for your user only (no
  administrator rights) with Start menu entries (*poetautopublish*, *New
  poem*), a `poet` command and an uninstaller, in Turkish or English. Prefer
  no install? Unzip `…-windows-portable.zip` and run `poetautopublish.exe`.
  The installer isn't code-signed, so Windows may warn about an unknown
  publisher: choose **More info → Run anyway**.
- **Settings travel with your poems.** Who you are, your pen names, the other
  authors and your week names now live in a `.poet/` folder inside the poems
  folder; existing settings are copied there once.
- **Sync through git (optional).** Make the poems folder a git repo with a
  remote, set `sync = true`, and the background service commits, pulls and
  pushes on its own, building poems that arrive. The PDFs go to the remote's
  `pdf` branch. A poem changed in two places keeps both versions. `poet sync`
  runs it once.
- New poems get file names Windows accepts.

**On Windows, for now**

Writing, signing, collections, publications, search and stats all work. PDFs
need XeLaTeX ([MiKTeX](https://miktex.org) or TeX Live); without it the app
still works and builds nothing. A built-in PDF engine and the background
rebuild at sign-in are planned.

**Linux**

Needs XeLaTeX (`texlive-xetex texlive-latexrecommended texlive-latexextra`)
and the Liberation fonts. Unpack the archive and put `poetautopublish` in
`~/.local/bin`; the first run walks you through the rest.

**Beta notes**

The build never changes your poem files, but keep a backup of your poems
folder. Please report problems in Issues. `SHA256SUMS` lists the checksums.
