# TouchBridge 배포 안내

TouchBridge는 Android 휴대폰을 여러 Windows PC에 연결해 제스처, 마우스, 키보드, 커스텀 버튼 입력을 전송하는 컨트롤러입니다. Windows Agent가 입력 실행을 담당하고, Android 앱은 제스처 패드, 마우스 패드, 키보드 리모트, 커스텀 버튼 UI를 제공합니다.

## 포함 파일

- `windows/TouchBridge-Agent-Windows.exe`
  - Windows PC에서 실행하는 Agent 앱입니다.
- `android/TouchBridge-Android-debug.apk`
  - 바로 설치 테스트가 가능한 Android APK입니다.
- `android/TouchBridge-Android-release-unsigned.apk`
  - 배포 서명 전 release APK입니다. 일반 사용자 설치용으로 배포하려면 별도 keystore로 서명해야 합니다.

## 현재 빌드 노트

- Android와 Windows Agent는 기존 JSON 대신 짧은 compact 프로토콜로 명령을 주고받습니다.
- BLE 마우스 입력은 낮은 지연을 위해 delta 메시지와 no-response write를 사용합니다.
- Android 키보드 입력창은 문자를 표시하지 않으며, 마스킹 문자도 그리지 않습니다.
- 긴 키보드 입력은 Android 전송과 Windows `SendInput` 실행 단계에서 작은 단위로 나뉩니다.

## 요구 사항

- Windows 10/11 PC
- Android 8.0 이상 권장
- BLE 사용 시 Windows와 Android 모두 Bluetooth LE 지원 필요
- USB 사용 시 USB 케이블 필요
- USB 케이블 네트워크 fallback 사용 시 Windows 방화벽에서 TouchBridge Agent의 private network 접근 허용 필요

## Windows Agent 설치 및 실행

1. `windows/TouchBridge-Agent-Windows.exe`를 Windows PC의 원하는 위치에 둡니다.
2. exe를 실행합니다.
3. Windows 보안 또는 방화벽 팝업이 뜨면 허용합니다.
4. 창의 X 버튼을 눌러도 Agent는 백그라운드에서 계속 동작합니다.
5. 다시 열려면 Windows 숨겨진 아이콘 영역의 TouchBridge 트레이 아이콘을 사용합니다.

## Android 앱 설치

테스트 설치:

1. Android 기기에 `android/TouchBridge-Android-debug.apk`를 복사합니다.
2. 파일 관리자에서 APK를 열어 설치합니다.
3. "알 수 없는 앱 설치" 권한이 필요하면 허용합니다.

정식 배포:

1. `android/TouchBridge-Android-release-unsigned.apk`를 배포용 keystore로 서명합니다.
2. 서명된 APK 또는 AAB를 배포 채널에 업로드합니다.

## 기본 사용법

1. Windows PC에서 TouchBridge Agent를 실행합니다.
2. Android 앱을 실행합니다.
3. 첫 화면에서 연결할 기기를 선택합니다.
4. 처음 연결하는 BLE 기기는 신뢰/페어링 확인을 허용합니다.
5. 연결되면 하단 탭에서 제스처, 버튼, 마우스, 키보드 기능을 선택합니다.
6. 제스처 영역에서는 탭, 더블 탭, 롱 프레스, 스와이프, 두 손가락/세 손가락 제스처를 입력합니다.
7. 마우스 패드에서는 포인터 이동, 스크롤, 왼쪽/오른쪽 클릭을 사용할 수 있습니다.
8. 키보드 탭에서는 휴대폰 키보드로 Windows에 텍스트를 보낼 수 있으며 입력창에는 작성한 문자가 표시되지 않습니다.

## BLE 연결

1. Windows Agent를 실행한 상태에서 Android 앱의 "새 기기 연결"을 누릅니다.
2. 주변 TouchBridge BLE 기기가 발견되면 목록에 표시됩니다.
3. 기기 카드를 누르고 신뢰 확인을 허용합니다.
4. 연결 성공 후 Gesture Pad를 사용합니다.

BLE가 보이지 않을 때:

- Windows Bluetooth가 켜져 있는지 확인합니다.
- Android Bluetooth 권한을 허용했는지 확인합니다.
- Windows Agent를 재시작한 뒤 다시 검색합니다.

## USB 연결

TouchBridge는 ADB를 사용하지 않습니다. Android SDK, adb, 개발자 옵션은 필요하지 않습니다.

1. Windows Agent를 실행합니다.
2. USB 케이블로 Android 기기와 PC를 연결합니다.
3. Android 앱의 USB 기기 카드를 누릅니다.
4. USB 권한 팝업이 뜨면 허용합니다.
5. AOA 직접 연결이 불가능한 기기에서는 Android USB 옵션에서 USB 테더링/케이블 네트워크를 켠 뒤 다시 연결합니다.

USB 연결이 실패할 때:

- Windows Agent 로그에 `USB cable network listener started on 0.0.0.0:47831`이 있는지 확인합니다.
- Windows 방화벽에서 TouchBridge Agent private network 접근을 허용합니다.
- Android에서 USB 테더링을 켠 뒤 다시 시도합니다.
- 충전 전용 케이블이 아닌 데이터 케이블을 사용합니다.

## 커스텀 버튼

1. Gesture Pad 화면의 `+` 버튼 또는 앱 설정에서 "버튼 추가"를 누릅니다.
2. Windows로 보낼 신호 ID와 Android에 표시할 버튼 이름을 입력합니다.
3. 연결 중이면 버튼 목록이 Windows Agent로 동기화됩니다.
4. Windows Agent에서 해당 버튼 ID에 단축키, Python 코드, PowerShell 스크립트를 매핑합니다.
5. Android에서 버튼을 누르면 Windows에서 매핑된 동작이 실행됩니다.

Python 동작은 Windows PC에 Python이 설치되어 있어야 저장/실행할 수 있습니다.

## 앱 설정

- 언어: System, English, 한국어 중 선택
- 제스처 진동: 제스처 전송 성공 시 휴대폰 진동 켜기/끄기
- 커스텀 버튼 추가

## 빌드 명령

Windows Agent:

```powershell
cd window_app\touchbridge-agent
cargo build --release
```

Android APK:

```powershell
cd android_app
.\gradlew.bat :app:assembleDebug
.\gradlew.bat :app:assembleRelease
```

## 알려진 배포 참고사항

- 현재 `release unsigned` APK는 서명 전 산출물입니다.
- Windows Agent는 별도 설치 프로그램이 아니라 단일 exe 산출물입니다.
- USB 직접 연결은 Windows/Android 드라이버 상태에 따라 제한될 수 있어 USB 케이블 네트워크 fallback을 함께 사용합니다.
