# ADR 0002. Три сборки iOS

Статус: принято  
Дата: 2026-10-27

## Контекст

Один бинарник не может одновременно пройти ревью App Store и содержать приватные символы. Пол Legacy — iOS 9.0, см. [ADR 0009](0009-legacy-floor-ios9.md). Xcode 15 не собирает armv7 и не ставит цель ниже iOS 12.

## Решение

Три таргета на общем дереве `ios/Sources`, язык везде Objective-C.

| Таргет | Инструменты | Куда ставится | Что умеет |
|---|---|---|---|
| `Span-Legacy` | Xcode 13.4, iOS 9.0, armv7+arm64 | sideload | камера, микрофон, пульт, показ рабочего стола |
| `Span-Modern` | Xcode 15+, iOS 15+, arm64 | App Store | то же плюс ReplayKit и HEVC, без фона камеры и без private API |
| `Span-Advanced` | тот же современный SDK | sideload или jailbreak | `IOMobileFramebuffer`, `IOHIDEventSystemClient`, accessibility-раннер |

Advanced загружает приватные символы через `dlopen` внутри своего таргета. Store-сборка на этот таргет не ссылается. Проект Legacy хранится в формате, который Xcode 15 не мигрирует при случайном открытии.

`InputSink` один интерфейс, три реализации. В общей сборке живёт только `SelfAppSink`. Две другие собираются в Advanced.

## Последствия

Сложность сопровождения Legacy — L, риск средний: отдельный IPA, App Store его не примет, сборка на Xcode 13.4 и macOS 12. Современный CI гоняет Rust и позже `Span-Modern`.
