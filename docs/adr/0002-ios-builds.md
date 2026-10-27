# ADR 0002. Три сборки iOS

Статус: принято  
Дата: 2026-10-27

## Контекст

Один бинарник не может одновременно пройти ревью App Store и содержать приватные символы. Xcode 15 не собирает armv7 и не ставит deployment target iOS 8. Legacy собирается только Xcode 11.7 на macOS 10.15.

## Решение

Три таргета на общем дереве `ios/Sources`, язык везде Objective-C.

| Таргет | Инструменты | Куда ставится | Что умеет |
|---|---|---|---|
| `Span-Legacy` | Xcode 11.7, iOS 8.0, armv7+arm64 | sideload | камера, микрофон, пульт, показ рабочего стола |
| `Span-Modern` | Xcode 15+, iOS 15+, arm64 | App Store | то же плюс ReplayKit и HEVC, без фона камеры и без private API |
| `Span-Advanced` | тот же современный SDK | sideload или jailbreak | `IOMobileFramebuffer`, `IOHIDEventSystemClient`, accessibility-раннер |

Advanced загружает приватные символы через `dlopen` внутри своего таргета. Store-сборка на этот таргет не ссылается. Проект Legacy хранится в формате, который Xcode 15 не мигрирует при случайном открытии.

`InputSink` один интерфейс, три реализации. В общей сборке живёт только `SelfAppSink`. Две другие собираются в Advanced.

## Последствия

Сложность сопровождения Legacy — L, риск высокий: отдельная машина сборки, отдельный IPA, App Store этот IPA не примет. Современный CI гоняет Rust и позже `Span-Modern`. Legacy-CI только на закреплённом образе Catalina.
