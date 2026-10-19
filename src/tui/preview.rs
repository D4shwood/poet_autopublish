//! In-terminal PDF page preview (sixel / kitty graphics), rendered with pdftoppm.

use anyhow::{Context, Result, bail};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Preview {
    pub pdf: PathBuf,
    /// 1-based
    pub page: usize,
    pub pages: usize,
    pub image: StatefulProtocol,
}

impl Preview {
    /// Opens the PDF at the page whose text starts with `title`, if given.
    pub fn open(picker: &Picker, pdf: &Path, title: Option<&str>) -> Result<Preview> {
        let text = Command::new("pdftotext")
            .arg(pdf)
            .arg("-")
            .output()
            .context("running pdftotext")?;
        let text = String::from_utf8_lossy(&text.stdout);
        // pdftotext ends every page with a form feed, so the last piece is empty.
        let pages: Vec<&str> = text.split('\x0c').collect();
        let count = pages.len().saturating_sub(1).max(1);
        let page = title
            .and_then(|t| {
                pages
                    .iter()
                    .position(|p| p.trim_start().starts_with(t.trim()))
            })
            .map(|i| i + 1)
            .unwrap_or(1);
        let image = render(picker, pdf, page)?;
        Ok(Preview {
            pdf: pdf.to_path_buf(),
            page,
            pages: count,
            image,
        })
    }

    pub fn turn(&mut self, picker: &Picker, delta: i32) -> Result<()> {
        let next = (self.page as i32 + delta).clamp(1, self.pages as i32) as usize;
        if next != self.page {
            self.image = render(picker, &self.pdf, next)?;
            self.page = next;
        }
        Ok(())
    }
}

fn render(picker: &Picker, pdf: &Path, page: usize) -> Result<StatefulProtocol> {
    let dir = tempfile::tempdir()?;
    let prefix = dir.path().join("page");
    let p = page.to_string();
    let status = Command::new("pdftoppm")
        .args(["-r", "110", "-png", "-singlefile", "-f", &p, "-l", &p])
        .arg(pdf)
        .arg(&prefix)
        .status()
        .context("running pdftoppm")?;
    if !status.success() {
        bail!("pdftoppm failed");
    }
    let img = image::open(prefix.with_extension("png"))?;
    Ok(picker.new_resize_protocol(img))
}
