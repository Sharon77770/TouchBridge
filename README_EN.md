# TouchBridge

TouchBridge turns an Android phone into a gesture controller for Windows PCs. The mobile app sends gestures and custom button events, and the Windows Agent receives them to execute hotkeys, Python code, PowerShell scripts, or other mapped actions.

## Features

- Android gesture pad
  - Tap, double tap, long press, and swipe
  - Two-finger tap and swipe
  - Three-finger tap
- Device selection for multiple Windows PCs
- BLE connection
- USB connection without ADB
  - Android Open Accessory direct connection attempt
  - USB tethering/cable network fallback
- Custom buttons
  - Add button IDs and display labels on Android
  - Sync button lists to the Windows Agent
  - Map each button to an action on Windows
- Windows action mappings
  - Hotkeys
  - Python code
  - PowerShell scripts
- Windows tray background operation
- Korean and English UI support
- Optional Android vibration after successful gesture sends

## Project Structure

```text
android_app/
  Android mobile app

window_app/touchbridge-agent/
  Windows Agent Rust app

release/
  Distribution artifacts and installation guides
```

## Quick Start

Distribution artifacts are available in the `release/` folder.

- Windows Agent: `release/windows/TouchBridge-Agent-Windows.exe`
- Android test APK: `release/android/TouchBridge-Android-debug.apk`
- Android release unsigned APK: `release/android/TouchBridge-Android-release-unsigned.apk`
- Korean release guide: `release/README_KO.md`
- English release guide: `release/README_EN.md`

`TouchBridge-Android-release-unsigned.apk` is not signed. Sign it with a release keystore before distributing it to end users.

## Usage

1. Run TouchBridge Agent on the Windows PC.
2. Install and open the Android app.
3. Select the PC to connect to from the first screen.
4. Approve the trust/pairing prompt for first-time BLE devices.
5. After connection, use gestures on the Gesture Pad screen.
6. Add custom buttons from the Gesture Pad `+` button or app settings if needed.
7. Map each custom button ID to an action in the Windows Agent.

## BLE Connection

1. Run the Windows Agent.
2. Tap "Connect new device" in the Android app.
3. Select a discovered TouchBridge BLE device.
4. Use the Gesture Pad after the connection succeeds.

If BLE discovery fails, check Windows/Android Bluetooth state and Android Bluetooth permissions.

## USB Connection

TouchBridge USB does not use ADB. Android SDK, adb, and Developer Options are not required.

1. Run the Windows Agent.
2. Connect the Android device to the PC with a USB cable.
3. Tap the USB device card in the Android app.
4. Allow the USB permission prompt if it appears.
5. If direct USB connection fails, enable USB tethering/cable network from Android USB options and try again.

If Windows Firewall prompts you, allow TouchBridge Agent on private networks so the USB cable network fallback can work.

## Development Build

### Windows Agent

```powershell
cd window_app\touchbridge-agent
cargo build
```

Release build:

```powershell
cd window_app\touchbridge-agent
cargo build --release
```

### Android App

```powershell
cd android_app
.\gradlew.bat :app:assembleDebug
```

Release unsigned APK:

```powershell
cd android_app
.\gradlew.bat :app:assembleRelease
```

## Updating The Release Folder

After building, copy these files into `release/`.

```text
window_app/touchbridge-agent/target/release/touchbridge-agent.exe
android_app/app/build/outputs/apk/debug/app-debug.apk
android_app/app/build/outputs/apk/release/app-release-unsigned.apk
```

The current release folder uses these names.

```text
release/windows/TouchBridge-Agent-Windows.exe
release/android/TouchBridge-Android-debug.apk
release/android/TouchBridge-Android-release-unsigned.apk
```

## Known Limitations

- The Android release APK is unsigned unless release signing is configured.
- Direct USB access can be limited by Windows/Android driver binding.
- USB fallback may require USB tethering/cable network and Windows Firewall permission.
- The Windows Agent is currently distributed as a standalone exe, not an installer.

## License

See the `LICENSE` file.
