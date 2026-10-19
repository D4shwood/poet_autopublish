# poet_autopublish

[English](README.md)

Şiirlerini terminalde sakin bir sayfada yaz; kendi serinin (benimkinin adı
*Contra Noir*) haftalık PDF ciltleri olarak otomatik yayımlansın.

![Bir cilt kapağı ve bir şiir sayfası](docs/pages.png)

- **Yaz**: dikkat dağıtmayan bir yazma ekranı. Satır sarma, daktilo modu,
  otomatik kayıt, taslaklar ve tek tuşla imzalama var.
- **Topla**: şiirler tek bir klasördeki düz metin dosyalarıdır. Son satırdaki
  imza şiirin tarihini ve hangi haftaya ait olduğunu belirler.
- **Yayımla**: her pazartesi–pazar haftası bir PDF cilt olur (kapak, sayfa
  başına bir şiir, künye). Ayrıca içindekiler sayfalı bir toplu kitap çıkar.
  Bir şiir değiştiğinde sadece değişen ciltler otomatik olarak yeniden derlenir.

Programın adı `poetautopublish`, kısa komutu `poet`.

## Kurulum

Rust ve birkaç yaygın paketle birlikte XeLaTeX gerekir. Arch / CachyOS'ta:

```sh
sudo pacman -S rust texlive-xetex texlive-latexrecommended texlive-latexextra ttf-liberation
cargo install --git https://github.com/D4shwood/poet_autopublish --root ~/.local
poetautopublish install
```

`install` şunları kurar:
- şiir klasörün her değiştiğinde PDF'leri yeniden derleyen bir systemd kullanıcı servisi,
- `~/.local/bin` içine `poet` komutu,
- *Yeni şiir* eylemli bir menü girdisi.

`poetautopublish uninstall` bunların hepsini kaldırır.

İlk çalıştırmada `~/.config/poetautopublish/config.toml` oluşturulur. En
azından `author` (imzaladığın adın tam hali) ve `poems_dir` ayarlarını yap. Tüm
seçenekler için [config.example.toml](config.example.toml) dosyasına bak.

## Şiir dosyaları

Şiir, uzantısız ya da `.txt` uzantılı bir UTF-8 dosyasıdır:

```
Şiirin başlığı

İlk mısra,
    girintili satırlar girintisini korur,

boş satır yeni bir kıta başlatır.

- Adın Soyadın. 13.10.2026 - 22.58
```

- **1. satır** başlıktır.
- **Son satır imzadır.** Sadece imzalı şiirler yayımlanır.
  `Adın 13/10/2026 22:58` gibi yazımlar ya da saatsiz bir tarih de olur.
- **Günden emin değil misin?** Tarihin önüne `~` koy: `- Adın Soyadın. ~08.10.2026`
  "8 Ekim civarı" demektir. Şiir o haftaya, kesin tarihli şiirlerden sonra
  girer. Alt bilgisinde sadece ayın kaçıncı haftası yazar
  (`Ekim 2026 · 2. hafta`).
- **Gece yazmak:** 05.00'ten önce imzalanan şiir bir önceki güne sayılır
  (`night_cutoff`), böylece o gecenin haftasında kalır. Sayfada yine gerçek
  saat görünür.
- **Taslaklar:** başlıktan sonraki satıra `#taslak` (ya da `#draft`) yazarsan
  şiir imzalı olsa bile yayımlanmaz.
- **Epigraflar:** adı `_epigraf` ile başlayan bir dosya, bir haftanın kapağına
  alıntı koyar. Son satırı hangi haftaya ait olduğunu belirleyen tarihtir
  (`13.10.2026`).
- **Revizyonlar:** bir şiiri imzaladıktan bir günden fazla sonra düzenlersen
  alt bilgisine `rev. <tarih>` eklenir.

Gizli dosyalar, `_` ile başlayan dosyalar ve yedek dosyaları yok sayılır.

## Koleksiyonlar

Haftalık seriye ait olmayan şiirler, örneğin eski şiirlerin, şiir klasörünün
bir **alt klasörüne** girer (`~/poetry/Eski Şiirler/` gibi). Her alt
klasör kendi kitabı olur: `eski_siirler.pdf`. Kapağında klasörün adı ve senin
adın yer alır; bir de içindekiler sayfası vardır. Şiirler alfabetik sıralanır
ve tıpkı seride olduğu gibi sadece imzalı olanlar yayımlanır. Koleksiyonlar
haftalık ciltlere ve serinin toplu kitabına karışmaz. Arayüzde *Koleksiyonlar*
başlığı altında listelenir.

## Çıktı

Bir hafta, en erken şiirinin tarihiyle adlandırılır ("5 Ekim Haftası"). Arayüzde
`r` ile yeniden adlandırabilirsin. Özel adlar
`~/.config/poetautopublish/weeks.toml` dosyasında tutulur. Kapağı, içindekileri
ve PDF başlığını değiştirir, dosya adını değiştirmez.

Her hafta için haftanın ilk şiirinin adını taşıyan bir
`<seri>_GG_AA_week.pdf` dosyası oluşur, örneğin `contra_noir_13_10_week.pdf`.
Toplu kitabın adı `<seri>_all.pdf`'tir ve en yeni hafta en başta yer alır.
`mirror_dir` ayarlarsan her PDF ayrıca başka bir klasöre de kopyalanır (örneğin
eşitlenen bir klasöre).

Şiirlerinin çoğu Türkçeyse PDF etiketleri ve arayüz Türkçe, değilse İngilizce
olur (`lang = "tr" | "en" | "auto"`).

## Kullanım

```sh
poet            # terminal arayüzü
poet new        # doğrudan boş sayfaya
poet build      # değişenleri derle (her şey için --force)
poet watch      # servisin çalıştırdığı komut
```

Arayüzde:

| Tuş | |
|---|---|
| `n` | yeni şiir |
| `r` | seçili haftayı yeniden adlandır (boş bırakırsan varsayılan ad geri gelir) |
| `⏎` | seçili şiiri yazma ekranında aç |
| `/` | tüm şiirlerde ve geçmişlerinde ara |
| `h` | seçili şiirin geçmişi |
| `o` | PDF'i aç (hangi programla açılacağını config'deki `pdf_viewer` belirler) |
| `b` / `f` | derle / her şeyi baştan derle |
| `D` | şiiri çöp kutusuna taşı |
| `s` | istatistikler |
| `w` | arka plan izleyicisini başlat, durdur ya da kur |
| `l` | kayıt |
| `e` | şiiri `$EDITOR` ile aç |
| `q` / **End Noir** | çık; süren bir derleme varsa önce bitmesi beklenir |

Yazma ekranında:
- `Ctrl+S` taslak olarak kaydeder.
- `Ctrl+G` şu anki saatle imzalar ve yayımlar.
- `Ctrl+T` daktilo modunu açıp kapatır.
- `Ctrl+Z` geri alır.
- `Esc` kaydedip geri döner.

Her kayıt yerel bir geçmişte saklanır
(`~/.local/share/poetautopublish/history`, şiir başına son 50 sürüm).

Arka plan servisi çalışmıyorsa, arayüz açık olduğu sürece klasörü kendisi izler.

## Görünümü özelleştirme

[templates/template.tex](templates/template.tex) dosyasını
`~/.config/poetautopublish/template.tex` konumuna kopyalayıp düzenle. Renkler,
fontlar, kenar boşlukları ve kapak düzeni orada. Yeniden derlemeye gerek yok.

Paketteki örnekle denemek için `author = "Örnek Şair"` ayarlayıp şunu çalıştır:

```sh
poet build --poems-dir examples/poems
```

## Lisans

Tüm hakları saklıdır.
