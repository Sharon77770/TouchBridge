# TouchBridge

TouchBridge turns an Android phone into a gesture, mouse, keyboard, and custom-button controller for Windows PCs.

TouchBridge는 Android 휴대폰을 Windows PC용 제스처, 마우스, 키보드, 커스텀 버튼 컨트롤러로 사용하는 앱입니다.

## Current Build Notes

- BLE and USB transports use the compact TouchBridge protocol instead of the previous JSON message format.
- BLE mouse input uses low-latency delta messages and no-response realtime writes where supported.
- Keyboard input is hidden on Android without masking characters in the text field.
- Keyboard text is chunked before transport and before Windows `SendInput` execution to avoid long-input stalls.

## Documentation

- [English README](README_EN.md)
- [한국어 README](README_KO.md)

## Release Guides

- [English release guide](release/README_EN.md)
- [한국어 배포 가이드](release/README_KO.md)

## Artifacts

- `release/windows/TouchBridge-Agent-Windows.exe`
- `release/android/TouchBridge-Android-debug.apk`
- `release/android/TouchBridge-Android-release-unsigned.apk`
