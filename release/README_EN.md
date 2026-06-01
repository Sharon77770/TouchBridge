# TouchBridge Release Guide

TouchBridge turns an Android phone into a gesture, mouse, keyboard, and custom-button controller for one or more Windows PCs. The Windows Agent receives commands and executes mapped actions, while the Android app provides gesture pad, mouse pad, keyboard remote, and custom button UI.

## Included Files

- `windows/TouchBridge-Agent-Windows.exe`
  - The Windows Agent application.
- `android/TouchBridge-Android-debug.apk`
  - An installable APK for testing.
- `android/TouchBridge-Android-release-unsigned.apk`
  - An unsigned release APK. Sign it with a distribution keystore before shipping it to end users.

## Current Build Notes

- Android and Windows Agent now exchange commands through the compact protocol instead of JSON.
- BLE mouse input uses low-latency delta messages and no-response writes.
- The Android keyboard input field does not render typed characters or mask glyphs.
- Long keyboard input is split before Android transport and before Windows `SendInput` execution.

## Requirements

- Windows 10/11 PC
- Android 8.0 or newer recommended
- Bluetooth LE support on both devices for BLE mode
- USB cable for USB mode
- Windows Firewall private network access for TouchBridge Agent when using the USB cable network fallback

## Install And Run The Windows Agent

1. Place `windows/TouchBridge-Agent-Windows.exe` anywhere on the Windows PC.
2. Run the exe.
3. Allow Windows Security or Firewall prompts if they appear.
4. Closing the window with the X button keeps the Agent running in the background.
5. Use the TouchBridge tray icon in the Windows hidden icons area to open it again.

## Install The Android App

For test installation:

1. Copy `android/TouchBridge-Android-debug.apk` to the Android device.
2. Open the APK from a file manager and install it.
3. Allow "Install unknown apps" if Android asks for it.

For production distribution:

1. Sign `android/TouchBridge-Android-release-unsigned.apk` with your release keystore.
2. Publish the signed APK or AAB through your distribution channel.

## Basic Usage

1. Start TouchBridge Agent on the Windows PC.
2. Open the Android app.
3. Select the PC to connect to from the first screen.
4. For a first-time BLE connection, approve the trust/pairing prompt.
5. After a successful connection, use the bottom tabs to choose gestures, buttons, mouse, or keyboard input.
6. On the gesture area, use tap, double tap, long press, swipe, two-finger gestures, and three-finger tap.
7. On the mouse pad, use pointer movement, scrolling, left click, and right click.
8. On the keyboard tab, type into Windows from the Android soft keyboard; typed text is not shown in the Android input field.

## BLE Connection

1. With Windows Agent running, tap "Connect new device" in the Android app.
2. Nearby TouchBridge BLE devices appear in the list.
3. Tap the device card and approve the trust prompt.
4. Use the Gesture Pad after the connection succeeds.

If BLE discovery fails:

- Check that Bluetooth is enabled on Windows.
- Check that Bluetooth permissions are allowed on Android.
- Restart Windows Agent and scan again.

## USB Connection

TouchBridge does not use ADB. Users do not need Android SDK, adb, or Developer Options.

1. Start Windows Agent.
2. Connect the Android device to the PC with a USB cable.
3. Tap the USB device card in the Android app.
4. Allow the USB permission prompt if it appears.
5. If direct AOA USB is not available on the device, enable USB tethering/cable network from the Android USB options and connect again.

If USB connection fails:

- Check the Windows Agent log for `USB cable network listener started on 0.0.0.0:47831`.
- Allow TouchBridge Agent on Windows Firewall private networks.
- Enable USB tethering on Android and retry.
- Use a data-capable USB cable, not a charge-only cable.

## Custom Buttons

1. Tap the `+` button on the Gesture Pad screen or use "Add button" from app settings.
2. Enter the signal ID sent to Windows and the button label shown on Android.
3. If connected, the button list is synced to Windows Agent.
4. In Windows Agent, map that button ID to a hotkey, Python code, or PowerShell script.
5. Pressing the custom button on Android executes the mapped action on Windows.

Python actions require Python to be installed on the Windows PC before they can be saved or run.

## App Settings

- Language: System, English, Korean
- Gesture vibration: turn phone vibration on/off after successful gesture sends
- Add custom buttons

## Build Commands

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

## Distribution Notes

- The `release unsigned` APK is not signed for end-user installation.
- The Windows Agent is currently provided as a standalone exe, not an installer.
- Direct USB access depends on Windows and Android driver binding, so TouchBridge also provides a USB cable network fallback.
