#!/bin/sh
# Build and install the Span agent onto a connected iPhone or iPad.
# This step runs only on a Mac with Xcode. Legacy devices need Xcode 13.4.
set -eu

if [ "$(uname -s)" != "Darwin" ] || ! command -v xcodebuild >/dev/null 2>&1; then
  echo "Установка на iPhone и iPad с этой машины невозможна." >&2
  echo "Нужен Mac:" >&2
  echo "  Legacy (iOS 9, armv7): Xcode 13.4 на macOS 12, устройство разблокировано и нажато «Доверять»." >&2
  echo "  Store и Advanced (iOS 15 / iPadOS 15): Xcode 15 или новее." >&2
  echo "Дальше на Mac: ./scripts/install-ios.sh legacy   или   ./scripts/install-ios.sh modern" >&2
  exit 2
fi

target="${1:-legacy}"
case "$target" in
  legacy) scheme="Span-Legacy" ;;
  modern) scheme="Span-Modern" ;;
  *)
    echo "Укажите legacy или modern." >&2
    exit 2
    ;;
esac

if ! command -v xcodegen >/dev/null 2>&1; then
  echo "Нужен XcodeGen: brew install xcodegen" >&2
  exit 2
fi

cd "$(dirname "$0")/../ios/Span"
xcodegen generate
xcodebuild \
  -project Span.xcodeproj \
  -scheme "$scheme" \
  -destination 'generic/platform=iOS' \
  -allowProvisioningUpdates \
  build

echo "Сборка $scheme готова. Установка на подключённое устройство:"
echo "  ios-deploy --bundle путь/к/Span.app"
echo "или кнопка Run в Xcode при выбранном устройстве."
echo "Бесплатная подпись Apple ID живёт 7 дней. App Store сборку Legacy не примет."
