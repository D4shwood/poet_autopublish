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
poetautopublish
```

İlk açılışta program seni adım adım tanır:
1. imzalarken kullandığın adın
2. rumuzların
3. şiir klasörü ve PDF klasörü (yoksa oluşturulur)
4. haftalık serinin adı
5. arka planda kendiliğinden derlensin mi

Son adıma evet dersen şunlar kurulur:
- şiir klasörün her değiştiğinde PDF'leri yeniden derleyen bir systemd kullanıcı servisi
- `~/.local/bin` içine `poet` komutu
- *Yeni şiir* eylemli bir menü girdisi

`poetautopublish uninstall` bunların hepsini kaldırır.

Menüdeki **⚙ Ayarlar** aynı ekranları cevapların dolu haliyle tekrar açar.
Cevaplar `~/.config/poetautopublish/config.toml` ve `authors.toml` dosyalarına
kaydedilir. Tüm seçenekler için [config.example.toml](config.example.toml)
dosyasına bak.

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
- **Rumuzlar ve diğer yazarlar.** Arayüzdeki *İmzalar* ekranı yazarları ve
  rumuzlarını listeler. Kendi adınla ya da rumuzlarından biriyle imzaladığın
  şiirler yayımlanır ve sayfada rumuz görünür. Başka yazarların şiirleri orada
  takip edilir ama yayımlanmaz. Kimseye ait olmayan imzalar da orada çıkar;
  ⏎ ile bir yazara bağlarsın. Tablo `~/.config/poetautopublish/authors.toml`
  dosyasında tutulur.
- **Tarih bilinmiyorsa:** tarihsiz bir imza da geçerlidir, yeter ki tanınan bir
  ad ya da rumuzla başlasın: `- K.M Ozan Deniz. Mürekkep #2`. Böyle şiirler
  koleksiyonlara ve Yayınlar ağacına girer, haftalık ciltlere giremez, çünkü
  hiçbir haftaya ait değildir.
- **Nerede yayımlandığı:** imza satırında tarihten (ve saatten) sonra yazılan
  her şey yayın bilgisidir, örneğin `- K.M Ozan. 31.10.2024 Mürekkep #0`.
  Tarihin altına *ilk yayın: Mürekkep #0* olarak basılır. Sayıyı `#`'den
  sonra, o baskının özel adını `:`'dan sonra yaz: `Mürekkep #0`,
  `Örnek Yayın #3 : Almanak` ya da `Örnek Yayın : Almanak`. Arayüzde
  **❖ Yayınlar**, her alanın yayımlanmış şiirlerini yayın → sayı → şiir ağacı
  olarak gösterir.
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
haftalık ciltlere ve serinin toplu kitabına karışmaz.

Klasörleri elle açman gerekmez. Arayüzde *Koleksiyonlar* başlığı altında boş
olanlar dahil hepsi listelenir. **+ Yeni koleksiyon** ad sorar ve klasörü
kendisi açar. Bir koleksiyon seçiliyken `n`'ye basarsan yeni şiir doğrudan o
koleksiyona yazılır. Koleksiyonun kitabı, içine ilk imzalı şiir girdiğinde
oluşur.

Bir koleksiyonun üstünde ⏎'ye basınca **o koleksiyona girersin**. Başlıkta
koleksiyonun adı yazar ve bütün arayüz ona ait olur: yeni şiirler oraya
kaydedilir; taslaklar, imzasızlar, arama ve istatistikler (haftalar yerine
yıllara göre) sadece o koleksiyonu kapsar. *Diğer alanlar* altındaki
**◆ Contra Noir** seni seriye geri götürür.

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
