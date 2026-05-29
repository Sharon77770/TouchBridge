# TouchBridge BLE Setup

TouchBridge uses BLE GATT for Android-to-Windows gesture transport.

1. Turn on Bluetooth on Windows and Android.
2. Start the Windows TouchBridge Agent.
3. Confirm the Windows UI shows BLE advertising as started.
4. Install and open the Android app.
5. Grant Bluetooth/Nearby devices permission when prompted.
6. Enter gestures on the touch pad.

The Windows agent exposes:

```text
Service UUID: 4f2b9d9c-39c1-4f35-8752-9f32fd325f61
Gesture characteristic UUID: 8a7f1168-48af-4f0d-95ee-7d6701f34b46
```

The Android app scans for that service, connects, and writes one UTF-8 JSON gesture payload to the gesture characteristic:

```json
{"type":"gesture","name":"swipe_left"}
```
