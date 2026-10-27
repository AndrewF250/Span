# Span

iPhone и iPad как периферия компьютера: камера, микрофон, второй экран, пульт.

Пол: iOS 9.0 (Legacy), iOS 15 / iPadOS 15 (Store и Advanced), Windows 10 1809, Ubuntu 22.04 и Debian 12.

Сейчас в дереве зафиксированы архитектурные решения и протокол major 1. Приёмника AirPlay в репозитории нет: см. [ADR 0004](docs/adr/0004-airplay-receiver.md).

## Документы

- [Архитектура](docs/architecture.md)
- [Ограничения платформ](docs/platform-limits.md)
- [Матрица совместимости](docs/compatibility.md)
- [Решения](docs/adr/README.md)
- [Протокол](protocol/spec.md)

## Проверка

Нужны Rust stable и `protoc`.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -q -p span-fuzzer -- --iters 1000
```

Ядро и протокол — MIT. Драйверы Windows, когда появятся, будут в отдельном каталоге со своей лицензией.
