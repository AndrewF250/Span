# Ограничения платформ

Факты, на которые опирается протокол. Если функция им противоречит, она помечается в матрице совместимости и не попадает в основную сборку.

## Сборка iOS

Xcode 12 не ставит deployment target ниже iOS 9, Xcode 14 — ниже iOS 11. Сборка iOS 8.0 armv7+arm64 возможна Xcode 11.7 на macOS 10.15. App Store такой бинарник не принимает. Распространение Legacy — sideload, enterprise или jailbreak.

Swift в агенте не используется. Общий код — Objective-C и C.

`@available`, сцены `UIScene` и `UIStackView` в Legacy недоступны: сцены с iOS 13, `UIStackView` с iOS 9. Сетка пульта на iOS 8 строится на Auto Layout и `UICollectionView` (оба есть с iOS 6).

## Захват и кодирование

| API | Минимум |
|---|---|
| `AVCaptureSession`, `AVAudioEngine` | iOS 8 |
| `VTCompressionSession`, `VTDecompressionSession` | iOS 8 |
| `kVTCompressionPropertyKey_RealTime`, запрет B-кадров | iOS 8 |
| Константный QP | нет в VideoToolbox ни на одной версии |
| HEVC encode | iOS 11 и чип A10 или новее |
| `RPScreenRecorder` | iOS 9, свой интерфейс и запись в файл |
| `startCaptureWithHandler` | iOS 11, приложение на экране |
| Broadcast Upload Extension | iOS 10, системный эфир с iOS 11 |
| `RPSystemBroadcastPickerView` | iOS 12 |
| ProRes | iOS 15.1, отдельные устройства |
| `UIImpactFeedbackGenerator` | iOS 10, нужен Taptic Engine |
| Network.framework | iOS 12 |
| CryptoKit | iOS 13 |
| Локальная сеть, `NSBonjourServices` | iOS 14 |
| Developer Mode для XCTest-раннера | iOS 16 |

4K на iOS 8 недостижим. Потолок лучших устройств этой линии — 1080p.

Камера после сворачивания для App Store не обещается. `UIBackgroundModes: audio` удерживает микрофон. Видеозахват система может оборвать. В Store-сборке флаг фона камеры выключен. В sideload он экспериментальный и подписан предупреждением.

`IOMobileFramebuffer` и `IOHIDEventSystemClient` из песочницы AltStore не открываются. Нужен jailbreak или платформенный entitlement. Это таргет Advanced, не «обычный sideload».

## Полоса raw NV12

Размер кадра — `width * height * 3 / 2`.

| Режим | Битрейт |
|---|---|
| 1920×1080 @ 30 | 746 Мбит/с |
| 1920×1080 @ 60 | 1,49 Гбит/с |
| 3840×2160 @ 30 | около 3 Гбит/с |

Практический потолок Lightning / USB 2 — примерно 240–320 Мбит/с. Raw 1080p30 туда не помещается. Режим без видимых потерь на таком кабеле — all-intra H.264 с высоким битрейтом, не raw и не QP=0. Raw разрешается только после замера канала и только если замер покрывает битрейт с запасом. На Lightning сессия этот режим не предлагает.

Фактическая частота `AVAudioSession` может остаться 44,1 кГц. В рукопожатии уходит частота после активации сессии, а не желаемые 48 кГц.

## Канал

usbmux пробрасывает TCP к порту, который слушает приложение. Датаграмм нет. Медиа по кабелю — тот же заголовок кадра внутри TLS, тип сообщения 100.

mDNS на Windows 10 1607 системным API не закрывался. Продуктовый пол всё равно 1809, а реализацию рассылки несёт сам хост, без установленного Bonjour.

## Хост

| Механизм | Минимум | Следствие для Span |
|---|---|---|
| IddCx 1.0 | Windows 10 1607 | слишком бедный; отдельный драйвер не пишем |
| IddCx 1.4, загружается на 1803+ | Windows 10 1803 | наш драйвер |
| Продуктовый пол GUI | Windows 10 1809 | WebView2 и IddCx вместе |
| WebView2 | Windows 10 SAC 1709 | Tauri на 1607 нельзя |
| `MFCreateVirtualCamera` | Windows 11 22000 | не используем: камера — свой AVStream |
| Виртуальный микрофон WASAPI | драйвер | пользовательского API на Windows 10 нет |
| `webkit2gtk-4.1` | Ubuntu 22.04, Debian 12 | Tauri 2 |
| evdi, v4l2loopback | вне дерева ядра | Secure Boot требует MOK |
| HEVC в Media Foundation | расширение из магазина Microsoft | не у каждого Windows 10; интерфейс говорит об этом |

OpenGL ES 2.0 — рендерер Extend на iOS 8, включая armv7. Metal есть с iOS 8 только на A7 и новее.

Задержка glass-to-glass 30 мс на панели 60 Гц не является гарантией: один кадр уже 16,7 мс. В интерфейсе показывается измерение.
