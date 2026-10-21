use crate::poem::Poem;
use chrono::{Datelike, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Tr,
    En,
}

const TR_MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];
const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const TR_STOPWORDS: [&str; 16] = [
    "ve", "bir", "bu", "ne", "mi", "mı", "ile", "gibi", "de", "da", "ki", "ben", "sen", "için",
    "ama", "çok",
];

/// A poem counts as Turkish if it has Turkish-only letters or enough Turkish stopwords.
pub fn is_turkish(text: &str) -> bool {
    if text.chars().any(|c| "ğĞşŞıİ".contains(c)) {
        return true;
    }
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();
    let hits = words
        .iter()
        .filter(|w| TR_STOPWORDS.contains(&w.as_str()))
        .count();
    hits >= 2
}

/// `setting` is the config value: "tr", "en" or "auto".
/// The system's language (LC_ALL, LC_MESSAGES, LANG): Turkish or English.
pub fn system_lang() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
        {
            return if v.to_lowercase().starts_with("tr") {
                Lang::Tr
            } else {
                Lang::En
            };
        }
    }
    Lang::En
}

pub fn resolve(setting: &str, poems: &[Poem]) -> Lang {
    match setting.trim().to_lowercase().as_str() {
        "tr" => Lang::Tr,
        "en" => Lang::En,
        _ => {
            // No poems to judge by yet: speak the system's language.
            if poems.is_empty() {
                return system_lang();
            }
            let tr = poems.iter().filter(|p| is_turkish(&p.raw)).count();
            if tr * 2 > poems.len() {
                Lang::Tr
            } else {
                Lang::En
            }
        }
    }
}

impl Lang {
    pub fn month(self, m: u32) -> &'static str {
        let i = (m as usize).saturating_sub(1).min(11);
        match self {
            Lang::Tr => TR_MONTHS[i],
            Lang::En => EN_MONTHS[i],
        }
    }

    /// "13 Ekim 2026" / "13 October 2026"
    pub fn date_long(self, d: NaiveDate) -> String {
        format!("{} {} {}", d.day(), self.month(d.month()), d.year())
    }

    /// Which Monday-started week of its month a date falls in:
    /// "Ekim 2026 · 2. hafta" / "October 2026 · week 2".
    pub fn month_week(self, d: NaiveDate) -> String {
        let offset = d.with_day(1).unwrap().weekday().num_days_from_monday();
        let n = (d.day() + offset - 1) / 7 + 1;
        match self {
            Lang::Tr => format!("{} {} · {n}. hafta", self.month(d.month()), d.year()),
            Lang::En => format!("{} {} · week {n}", self.month(d.month()), d.year()),
        }
    }

    /// "20 Ekim" / "20 October"
    pub fn date_short(self, d: NaiveDate) -> String {
        format!("{} {}", d.day(), self.month(d.month()))
    }

    /// "13 Ekim Haftası" / "Week of 13 October"
    pub fn week_label(self, d: NaiveDate) -> String {
        match self {
            Lang::Tr => format!("{} {} Haftası", d.day(), self.month(d.month())),
            Lang::En => format!("Week of {} {}", d.day(), self.month(d.month())),
        }
    }

    pub fn volume(self, n: usize) -> String {
        match self {
            Lang::Tr => format!("Cilt {n}"),
            Lang::En => format!("Vol. {n}"),
        }
    }

    pub fn poem_count(self, n: usize) -> String {
        match self {
            Lang::Tr => format!("{n} şiir"),
            Lang::En if n == 1 => "1 poem".into(),
            Lang::En => format!("{n} poems"),
        }
    }

    /// Text strings for the PDFs and the TUI.
    pub fn t(self, key: &str) -> &'static str {
        let (tr, en) = match key {
            "contents" => ("İçindekiler", "Contents"),
            "compiled" => ("derlendi", "compiled"),
            "rev" => ("rev.", "rev."),
            "first_published" => ("ilk yayın:", "first published in"),
            "all_poems" => ("Bütün şiirler", "All poems"),
            "new_poem" => ("✎ Yeni şiir", "✎ New poem"),
            "continue_draft" => ("↺ Taslağa devam", "↺ Continue a draft"),
            "unsigned" => ("⚠ İmzasız", "⚠ Unsigned"),
            "drafts" => ("✎ Taslaklar", "✎ Drafts"),
            "stats" => ("▤ İstatistikler", "▤ Statistics"),
            "quit_item" => ("■ End Noir", "■ End Noir"),
            "quitting" => (
                "Kapanıyor… derleme bitince çıkılacak.",
                "Quitting… waiting for the build to finish.",
            ),
            "rename" => ("Haftanın yeni adı", "New name for the week"),
            "rename_keys" => (
                "⏎ kaydet · boş = varsayılan ad · Esc vazgeç",
                "⏎ save · empty = default name · Esc cancel",
            ),
            "weeks" => ("Haftalar", "Weeks"),
            "collections" => ("Koleksiyonlar", "Collections"),
            "other_spaces" => ("Diğer alanlar", "Other spaces"),
            "signatures" => ("◇ İmzalar", "◇ Signatures"),
            "settings" => ("⚙ Ayarlar", "⚙ Settings"),
            "publications" => ("❖ Yayınlar", "❖ Publications"),
            "no_publications" => (
                "Henüz yayın yok. İmzanın sonuna yayını yaz: - Ad. 31.10.2024 Mürekkep #0",
                "No publications yet. Write it after the signature: - Name. 31.10.2024 Journal #0",
            ),
            "main_author" => ("ana yazar · yayımlanır", "main author · published"),
            "tracked" => ("takip edilir · yayımlanmaz", "tracked · not published"),
            "no_real_name" => ("(adı yok)", "(no real name)"),
            "unknown_signatures" => ("Tanınmayan imzalar", "Unknown signatures"),
            "assign_hint" => ("⏎ bir yazara bağla", "⏎ assign to an author"),
            "assign_to" => ("kimin imzası?", "is whose signature?"),
            "new_author" => ("+ Yeni yazar", "+ New author"),
            "sig_keys" => (
                "⏎ seç · p rumuz ekle · x sil · Esc geri",
                "⏎ select · p add pen name · x remove · Esc back",
            ),
            "author_name" => (
                "Yazarın adı (boş bırakabilirsin)",
                "Author's name (may be empty)",
            ),
            "first_pen" => (
                "İlk rumuzu (boş bırakabilirsin)",
                "First pen name (may be empty)",
            ),
            "add_pen" => ("Yeni rumuz", "New pen name"),
            "prompt_keys" => ("⏎ tamam · Esc vazgeç", "⏎ ok · Esc cancel"),
            "name_taken" => (
                "Bu ad zaten bir yazara ait.",
                "That name already belongs to an author.",
            ),
            "confirm_remove_author" => ("Bu yazar silinsin mi? (e/h)", "Remove this author? (y/n)"),
            "per_year" => ("Yıllara göre şiir", "Poems per year"),
            "new_collection" => ("+ Yeni koleksiyon", "+ New collection"),
            "new_in_collection" => (
                "Henüz şiir yok · n: bu koleksiyona yeni şiir",
                "No poems yet · n: new poem in this collection",
            ),
            "collection_name" => ("Koleksiyonun adı", "Collection name"),
            "collection_keys" => ("⏎ oluştur · Esc vazgeç", "⏎ create · Esc cancel"),
            "poems" => ("Şiirler", "Poems"),
            "log" => ("Kayıt", "Log"),
            "watch_service" => ("● servis izliyor", "● service watching"),
            "watch_tui" => (
                "◐ TUI izliyor — w: servisi kur",
                "◐ TUI watching — w: install service",
            ),
            "watch_off" => ("○ izleme kapalı", "○ not watching"),
            "building" => ("derleniyor…", "building…"),
            "keys_main" => (
                "n yeni · ⏎ aç · r haftayı adlandır · / ara · h geçmiş · o pdf · b derle · f zorla · D çöpe · s istatistik · w izleyici · l kayıt · q çık",
                "n new · ⏎ open · r rename week · / search · h history · o pdf · b build · f force · D trash · s stats · w watcher · l log · q quit",
            ),
            "lines" => ("mısra", "lines"),
            "words" => ("kelime", "words"),
            "min" => ("dk", "min"),
            "saved" => ("kaydedildi", "saved"),
            "unsaved" => ("kaydedilmedi", "unsaved"),
            "draft" => ("taslak", "draft"),
            "signed" => ("imzalı", "signed"),
            "writer_keys" => (
                "^S kaydet · ^G imzala · ^T daktilo · ^E ölçü · Esc geri",
                "^S save · ^G sign · ^T typewriter · ^E meter · Esc back",
            ),
            "free_verse" => ("serbest", "free verse"),
            "now" => ("şimdi", "now"),
            "version" => ("sürüm", "version"),
            "history_keys" => (
                "↑↓ sürüm · v şimdiyle karşılaştır · Space oynat · ←→ adım · Esc geri",
                "↑↓ version · v compare with now · Space replay · ←→ step · Esc back",
            ),
            "syllables" => ("hece", "syllables"),
            "confirm_sign" => ("İmzalayıp yayımlansın mı? (e/h)", "Sign and publish? (y/n)"),
            "confirm_force" => (
                "Her şey yeniden derlensin mi? (e/h)",
                "Rebuild everything? (y/n)",
            ),
            "confirm_trash" => ("Çöp kutusuna taşınsın mı? (e/h)", "Move to Trash? (y/n)"),
            "search" => ("Ara", "Search"),
            "history" => ("Geçmiş", "History"),
            "no_history" => ("Henüz geçmiş yok.", "No history yet."),
            "no_pdf" => ("Bu seçim için PDF yok.", "No PDF for this selection."),
            "watcher" => ("İzleyici", "Watcher"),
            "watcher_menu" => (
                "1 başlat · 2 durdur · 3 yeniden başlat · 4 kur · Esc geri",
                "1 start · 2 stop · 3 restart · 4 install · Esc back",
            ),
            "empty" => ("(boş)", "(empty)"),
            "preview_keys" => (
                "← → sayfa · ↑ ↓ kaydır · Home/End baş/son · Esc geri",
                "← → page · ↑ ↓ scroll · Home/End first/last · Esc back",
            ),
            "total" => ("Toplam", "Total"),
            "streak" => ("Seri (gün)", "Streak (days)"),
            "per_week" => ("Haftalık şiir", "Poems per week"),
            "by_hour" => ("Yazdığın saatler", "Hours you write"),
            "foreign" => ("başka yazar", "other author"),
            _ => ("?", "?"),
        };
        match self {
            Lang::Tr => tr,
            Lang::En => en,
        }
    }

    pub fn yes(self, c: char) -> bool {
        match self {
            Lang::Tr => matches!(c, 'e' | 'E' | 'y' | 'Y'),
            Lang::En => matches!(c, 'y' | 'Y'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection() {
        assert!(is_turkish("Kış geldi, örttü bahçeyi."));
        assert!(is_turkish("Rahipler ve bir şövalye"));
        assert!(!is_turkish("I will walk the shore before the morning tide"));
    }

    #[test]
    fn labels() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        assert_eq!(Lang::Tr.week_label(d), "8 Ekim Haftası");
        assert_eq!(Lang::En.week_label(d), "Week of 8 October");
        assert_eq!(Lang::Tr.date_long(d), "8 Ekim 2026");
        assert_eq!(Lang::En.poem_count(1), "1 poem");
        // 1 October 2026 is a Thursday, so 1–4 Oct is week 1 and 8 Oct is week 2.
        assert_eq!(Lang::Tr.month_week(d), "Ekim 2026 · 2. hafta");
        let first = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        assert_eq!(Lang::En.month_week(first), "October 2026 · week 1");
    }
}
