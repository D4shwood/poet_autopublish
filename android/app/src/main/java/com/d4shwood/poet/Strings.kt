package com.d4shwood.poet

/**
 * Strings only the phone needs. Everything the terminal also shows comes from
 * the shared table in src/lang.rs, through `PoetCore.t` / `tr`.
 */
object S {
    private val table = mapOf(
        "welcome" to ("Hoş geldin" to "Welcome"),
        "step" to ("adım %d/%d" to "step %d/%d"),
        "q_name" to ("Adın nedir?" to "What is your name?"),
        "h_name" to (
            "Şiirlerini imzalarken kullandığın gerçek adın. Bu adla imzalanan şiirlerin yayımlanır." to
                "The real name you sign your poems with. Poems signed with it get published."
            ),
        "q_pens" to ("Rumuzların var mı?" to "Do you have pen names?"),
        "h_pens" to (
            "Virgülle ayır. Bu adlarla imzaladıkların da senin sayılır. Boş bırakabilirsin." to
                "Separate them with commas. Poems signed with them count as yours too. You can leave this empty."
            ),
        "q_series" to ("Haftalık serinin adı ne olsun?" to "What should the weekly series be called?"),
        "h_series" to (
            "Her hafta bir cilt olur; bu ad kapaklarda büyük harflerle yazar." to
                "Every week becomes a volume; this name is printed large on the covers."
            ),
        "q_folder" to ("Şiirlerin hangi klasörde?" to "Which folder holds your poems?"),
        "h_folder" to (
            "Her şiir bu klasörde bir metin dosyasıdır; alt klasörler koleksiyon olur." to
                "Each poem is a text file in this folder; subfolders become collections."
            ),
        "pick_folder" to ("⏎ klasör seç" to "⏎ choose a folder"),
        "q_summary" to ("Hazır. Kaydedilsin mi?" to "Ready. Save it?"),
        "h_summary" to ("Sonra ⚙ Ayarlar'dan değiştirebilirsin." to "You can change these later from ⚙ Settings."),
        "name" to ("Ad" to "Name"),
        "pens" to ("Rumuzlar" to "Pen names"),
        "series" to ("Seri" to "Series"),
        "folder" to ("Klasör" to "Folder"),
        "next" to ("⏎ ileri" to "⏎ next"),
        "back" to ("← geri" to "← back"),
        "save" to ("⏎ kaydet" to "⏎ save"),
        "required" to ("Bu boş kalamaz." to "This can't be empty."),
        "settings_title" to ("Ayarlar" to "Settings"),

        "k_new" to ("n yeni" to "n new"),
        "k_search" to ("/ ara" to "/ search"),
        "k_rename" to ("r adlandır" to "r rename"),
        "k_refresh" to ("⟳ yenile" to "⟳ refresh"),
        "k_back" to ("← geri" to "← back"),
        "k_open" to ("⏎ aç" to "⏎ open"),
        "k_trash" to ("D sil" to "D delete"),
        "k_add_pen" to ("p rumuz ekle" to "p add pen name"),
        "k_remove" to ("x sil" to "x remove"),
        "k_assign" to ("⏎ yazara bağla" to "⏎ assign"),
        "yes" to ("e evet" to "y yes"),
        "no" to ("h hayır" to "n no"),
        "ok" to ("⏎ tamam" to "⏎ ok"),
        "cancel" to ("Esc vazgeç" to "Esc cancel"),

        "w_save" to ("✓ kaydet" to "✓ save"),
        "w_sign" to ("✎ imzala" to "✎ sign"),
        "w_undo" to ("↶ geri al" to "↶ undo"),
        "w_typewriter" to ("⌨ daktilo" to "⌨ typewriter"),
        "w_meter" to ("♩ ölçü" to "♩ meter"),
        "w_back" to ("← geri" to "← back"),
        "w_title" to ("Başlık" to "Title"),

        "confirm_delete" to ("Bu şiir silinsin mi? (e/h)" to "Delete this poem? (y/n)"),
        "deleted" to ("silindi: %s" to "deleted: %s"),
        "renamed" to ("adlandırıldı: %s" to "renamed: %s"),
        "reading" to ("şiirler okunuyor…" to "reading poems…"),
        "read_failed" to ("Şiirler okunamadı: %s" to "Couldn't read the poems: %s"),
        "local" to ("○ yerel klasör" to "○ local folder"),
        "search_prompt" to ("/ " to "/ "),
        "no_hits" to ("(sonuç yok)" to "(no results)"),
        "syncing" to ("eşitleniyor…" to "syncing…"),
        "synced" to ("eşitlendi" to "synced"),
        "offline" to ("çevrimdışı" to "offline"),
        "conflict" to (
            "%s iki tarafta da değişmiş; telefondaki “(telefon)” olarak saklandı." to
                "%s changed on both sides; the phone's copy is kept as “(telefon)”."
            ),
        "k_pdf" to ("o pdf" to "o pdf"),
        "where_github" to ("GitHub deposu · bilgisayarla eşitlenir" to "GitHub repo · syncs with the computer"),
        "where_folder" to ("Telefondaki bir klasör" to "A folder on the phone"),
        "q_where" to ("Şiirlerin nerede?" to "Where are your poems?"),
        "h_where" to (
            "GitHub'daki özel depo bilgisayarındaki klasörle eşitlenir; PDF'ler de telefona gelir." to
                "A private GitHub repo syncs with the folder on your computer; the PDFs come to the phone too."
            ),
        "q_repo" to ("Hangi depo?" to "Which repo?"),
        "h_repo" to ("sahip/ad biçiminde, örneğin D4shwood/poems." to "As owner/name, e.g. D4shwood/poems."),
        "q_token" to ("Erişim anahtarı (token)" to "Access token"),
        "h_token" to (
            "GitHub › Settings › Developer settings › Fine-grained tokens: yalnızca bu depo, Contents: Read and write. Telefonda şifreli saklanır." to
                "GitHub › Settings › Developer settings › Fine-grained tokens: only this repo, Contents: Read and write. Stored encrypted on the phone."
            ),
        "token_kept" to ("(kayıtlı · değiştirmek için yaz)" to "(saved · type to replace)"),
        "repo" to ("Depo" to "Repo"),
    )

    fun get(lang: String, key: String): String =
        table[key]?.let { if (lang == "en") it.second else it.first } ?: key
}
