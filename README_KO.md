# TouchBridge

TouchBridge는 Android 휴대폰을 Windows PC용 제스처 컨트롤러로 사용하는 앱입니다. 모바일 앱에서 제스처나 커스텀 버튼을 입력하면 Windows Agent가 이를 받아 단축키, Python 코드, PowerShell 스크립트 같은 동작으로 실행합니다.

## 주요 기능

- Android 제스처 패드
  - 탭, 더블 탭, 롱 프레스, 스와이프
  - 두 손가락 탭/스와이프
  - 세 손가락 탭
- 여러 Windows PC 선택 및 연결
- BLE 연결
- ADB 없는 USB 연결
  - Android Open Accessory 직접 연결 시도
  - USB 테더링/케이블 네트워크 fallback
- 커스텀 버튼
  - Android에서 버튼 ID와 표시 이름 추가
  - Windows Agent로 버튼 목록 동기화
  - Windows에서 버튼별 동작 매핑
- Windows 동작 매핑
  - 단축키
  - Python 코드
  - PowerShell 스크립트
- Windows 트레이 백그라운드 실행
- 한국어/영어 UI 지원
- 제스처 전송 성공 시 Android 진동 설정

## 프로젝트 구조

```text
android_app/
  Android 모바일 앱

window_app/touchbridge-agent/
  Windows Agent Rust 앱

release/
  배포용 산출물과 설치 가이드
```

## 빠른 실행

배포 산출물은 `release/` 폴더에 있습니다.

- Windows Agent: `release/windows/TouchBridge-Agent-Windows.exe`
- Android 테스트 APK: `release/android/TouchBridge-Android-debug.apk`
- Android release unsigned APK: `release/android/TouchBridge-Android-release-unsigned.apk`
- 한국어 배포 가이드: `release/README_KO.md`
- English release guide: `release/README_EN.md`

`TouchBridge-Android-release-unsigned.apk`는 서명되지 않은 release APK입니다. 실제 사용자 배포 전 별도 keystore로 서명해야 합니다.

## 사용 방법

1. Windows PC에서 TouchBridge Agent를 실행합니다.
2. Android 앱을 설치하고 실행합니다.
3. 첫 화면에서 연결할 PC를 선택합니다.
4. 처음 보는 BLE 기기는 신뢰/페어링 확인을 허용합니다.
5. 연결되면 Gesture Pad 화면에서 제스처를 입력합니다.
6. 커스텀 버튼이 필요하면 Gesture Pad의 `+` 버튼 또는 앱 설정에서 추가합니다.
7. Windows Agent에서 커스텀 버튼 ID에 원하는 동작을 매핑합니다.

## BLE 연결

1. Windows Agent를 실행합니다.
2. Android 앱에서 "새 기기 연결"을 누릅니다.
3. 발견된 TouchBridge BLE 기기를 선택합니다.
4. 연결 성공 후 Gesture Pad를 사용합니다.

BLE 검색이 안 되면 Windows/Android Bluetooth 상태와 Android 권한을 확인하세요.

## USB 연결

TouchBridge USB 연결은 ADB를 사용하지 않습니다. Android SDK, adb, 개발자 옵션이 필요하지 않습니다.

1. Windows Agent를 실행합니다.
2. Android 기기와 PC를 USB 케이블로 연결합니다.
3. Android 앱에서 USB 기기 카드를 누릅니다.
4. USB 권한 팝업이 뜨면 허용합니다.
5. 직접 USB 연결이 실패하면 Android USB 옵션에서 USB 테더링/케이블 네트워크를 켠 뒤 다시 시도합니다.

Windows 방화벽 팝업이 뜨면 TouchBridge Agent의 private network 접근을 허용해야 USB 케이블 네트워크 fallback이 동작합니다.

## 개발 빌드

### Windows Agent

```powershell
cd window_app\touchbridge-agent
cargo build
```

Release 빌드:

```powershell
cd window_app\touchbridge-agent
cargo build --release
```

### Android 앱

```powershell
cd android_app
.\gradlew.bat :app:assembleDebug
```

Release unsigned APK:

```powershell
cd android_app
.\gradlew.bat :app:assembleRelease
```

## 배포 폴더 갱신

빌드 후 다음 파일을 `release/`에 복사합니다.

```text
window_app/touchbridge-agent/target/release/touchbridge-agent.exe
android_app/app/build/outputs/apk/debug/app-debug.apk
android_app/app/build/outputs/apk/release/app-release-unsigned.apk
```

현재 배포 폴더는 다음 이름을 사용합니다.

```text
release/windows/TouchBridge-Agent-Windows.exe
release/android/TouchBridge-Android-debug.apk
release/android/TouchBridge-Android-release-unsigned.apk
```

## 알려진 제약

- Android release APK는 서명 설정이 없으면 unsigned로 생성됩니다.
- USB 직접 연결은 Windows/Android 드라이버 바인딩 상태에 따라 제한될 수 있습니다.
- USB fallback은 USB 테더링/케이블 네트워크와 Windows 방화벽 허용이 필요할 수 있습니다.
- Windows Agent는 현재 설치 프로그램이 아닌 단일 exe 형태입니다.

## 라이선스

라이선스 정보는 `LICENSE` 파일을 확인하세요.
