# Arweak

Oyunlar için Windows performans aracı. Tek tuşla gerçekten etkisi olan optimizasyonları uygular, FPS'i ve takılmaları ölçer, her değişikliği tek tuşla geri alır.

- **Boost**: Seçili optimizasyonları uygular; oyun açılınca otomatik devreye girebilir
- **Oyunlar**: Fortnite, Valorant, CS2, Apex Legends, League of Legends, Minecraft, GTA V, Rocket League profilleri; yüklü oyunları kendisi bulur
- **Performans**: Ortalama FPS, %1 low, kare süresi, takılma sayısı ve Boost öncesi/sonrası karşılaştırma ([PresentMon](https://github.com/GameTechDev/PresentMon) ile)
- **Sistem**: Tek kanal RAM, EXPO/XMP, yenileme hızı, Oyun Modu, HAGS ve güç planı kontrolü
- **RAM temizleme**: Elle veya oyun sırasında bellek azaldığında otomatik
- Arka planda çalışır, Windows ile başlar, tepsiye küçülür

## İndirme

[Releases](../../releases) sayfasından `Arweak.exe` (kurulumsuz) veya `Arweak_1.0.0_Setup.exe` (kurulum sihirbazı).

Gereksinim: Windows 10/11 (64 bit). Yönetici izni ister.

## Optimizasyonlar

| Optimizasyon | Ne yapar | Yeniden başlatma |
|---|---|---|
| Ultimate güç planı | Windows'un gizli "Ultimate Performance" planını açar | — |
| Game Bar kaydını kapat | Arka planda sürekli yapılan oyun kaydını kapatır | — |
| Oyuna yüksek öncelik | Oyun açılınca işlemine yüksek öncelik verir | — |
| Oyunda arka planı kıs | Oyun sırasında tarayıcı, OneDrive gibi uygulamaların önceliğini düşürür | — |
| GPU maksimum performans | PCIe güç tasarrufunu kapatır, oyunlara yüksek performanslı GPU tercihini atar | — |
| Windows Oyun Modu | Oyun Modunu açar | — |
| Donanım hızlandırmalı zamanlama | HAGS'ı açar | ✓ |
| Çoklu düzlem kaplamasını kapat | MPO'yu kapatır; bazı sistemlerde titremeyi giderir | ✓ |
| Akıllı RAM temizleme | Oyun sırasında boş bellek azalınca Windows önbelleğini boşaltır | — |
| Ağ güç tasarrufunu kapat | Ağ kartının güç tasarrufunu kapatır | — |
| Nagle algoritmasını kapat | TCP paketlerinin bekletilmesini kapatır | — |
| Yüksek çözünürlüklü zamanlayıcı | Zamanlayıcı çözünürlüğünü sistem geneline yayar (Windows 11) | ✓ |

RAM temizleme yalnızca Windows'un **önbelleğini** (standby) boşaltır; programların belleğini zorla diske itmez. Bu yüzden oyunda takılmaya yol açmaz.

## Geri alma

Arweak her değişiklikten önce eski değeri `C:\ProgramData\Arweak\backup.json` dosyasına kaydeder. Boost kapatıldığında veya **Ayarlar → Tüm değişiklikleri geri al** ile her şey eski haline döner. Ayrıca **Yedek noktası oluştur** ile Windows sistem geri yükleme noktası alınabilir.

## Hile koruması olan oyunlar

Arweak oyunların dosyalarına veya belleğine dokunmaz; yalnızca Windows ve sürücü ayarlarını değiştirir ve işlem önceliği verir. Valorant gibi çekirdek seviyesinde koruması olan oyunlarda öncelik ayarı koruma tarafından engellenebilir; bu durumda o ayar sessizce atlanır.

## Kaynak koddan derleme

Gerekenler: Node.js 18+, Rust (stable, MSVC), Visual Studio Build Tools ("C++ ile masaüstü geliştirme").

```bash
npm install
npm run build
```

Çıktılar `release/` klasörüne kopyalanır. Font ve ikon dosyalarını yeniden üretmek için `npm run assets` ve `npm run icons`.

## Proje yapısı

| Yol | İçerik |
|---|---|
| `ui/` | Arayüz (HTML/CSS/JS); Tauri dışında açılırsa sahte verilerle çalışır |
| `src-tauri/src/main.rs` | Uygulama durumu, arayüz komutları, tepsi |
| `src-tauri/src/tweaks.rs` | Optimizasyonlar ve geri alma kayıtları |
| `src-tauri/src/monitor.rs` | İşlemci/GPU/RAM ölçümü, oyun algılama, PresentMon, çalışma anı optimizasyonları |
| `src-tauri/src/games.rs` | Oyun tanımları; Steam, Epic, Riot kurulum algılama |
| `src-tauri/src/sysinfo.rs` | Sistem kontrolü ve skor |
| `src-tauri/src/memory.rs` | Bellek durumu ve önbellek temizleme |
| `src-tauri/tools/PresentMon.exe` | FPS ölçümü için exe'ye gömülen araç |

## Lisans

[GPL-3.0-or-later](LICENSE). Üçüncü taraf bileşenler: [THIRD_PARTY.md](THIRD_PARTY.md)
