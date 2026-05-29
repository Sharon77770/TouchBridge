package com.sharon77770.touchbridge

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.res.Configuration
import android.hardware.usb.UsbManager
import android.os.Build
import android.os.LocaleList
import android.os.Bundle
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.sharon77770.touchbridge.ui.theme.TouchBridgeTheme
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlin.math.abs
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val TAG = "TouchBridgeApp"
private const val ACTION_USB_STATE = "android.hardware.usb.action.USB_STATE"
const val TOUCHBRIDGE_PREFS = "touchbridge_settings"
private const val PREF_GESTURE_VIBRATION_ENABLED = "gesture_vibration_enabled"
private const val PREF_APP_LANGUAGE = "app_language"
private const val GESTURE_VIBRATION_MS = 36L

private val AppBackground = Color(0xFF050A12)
private val AppSurface = Color(0xFF0D1420)
private val AppSurfaceHigh = Color(0xFF121C2A)
private val AppOutline = Color(0xFF263447)
private val AppGreen = Color(0xFF38D996)
private val AppRed = Color(0xFFFF6B6B)
private val AppAmber = Color(0xFFFFC857)
private val AppBlue = Color(0xFF8AB4FF)

class MainActivity : ComponentActivity() {
    private var permissionsGranted by mutableStateOf(false)

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { result ->
        permissionsGranted = result.values.all { it }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        applyPlatformAppLanguage(this, loadAppLanguageSetting(this))
        enableEdgeToEdge()
        Log.i(TAG, "Android client build=$TOUCHBRIDGE_ANDROID_CLIENT_BUILD")
        permissionsGranted = requiredBluetoothPermissions().all { permission ->
            checkSelfPermission(permission) == android.content.pm.PackageManager.PERMISSION_GRANTED
        }

        setContent {
            TouchBridgeTheme(darkTheme = true) {
                TouchBridgeApp(
                    permissionsGranted = permissionsGranted,
                    onRequestPermissions = {
                        permissionLauncher.launch(requiredBluetoothPermissions())
                    },
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TouchBridgeApp(
    permissionsGranted: Boolean = true,
    onRequestPermissions: () -> Unit = {},
    client: TouchBridgeClient? = null,
) {
    val baseContext = LocalContext.current
    var appLanguage by rememberSaveable {
        mutableStateOf(loadAppLanguageSetting(baseContext))
    }
    val localizedContext = remember(baseContext, appLanguage) {
        localizedContext(baseContext, appLanguage)
    }

    CompositionLocalProvider(LocalContext provides localizedContext) {
        TouchBridgeAppContent(
            permissionsGranted = permissionsGranted,
            onRequestPermissions = onRequestPermissions,
            client = client,
            appLanguage = appLanguage,
            onAppLanguageChange = { nextLanguage ->
                appLanguage = nextLanguage
                saveAppLanguageSetting(baseContext, nextLanguage)
                applyPlatformAppLanguage(baseContext, nextLanguage)
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TouchBridgeAppContent(
    permissionsGranted: Boolean,
    onRequestPermissions: () -> Unit,
    client: TouchBridgeClient?,
    appLanguage: AppLanguageSetting,
    onAppLanguageChange: (AppLanguageSetting) -> Unit,
) {
    val context = LocalContext.current
    val connectionManager = remember(context, client) {
        ConnectionManager(
            bleTransport = BleTransport(client ?: TouchBridgeClient(context)),
            usbTransport = UsbTransport(context),
        )
    }
    val scope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    var devices by remember {
        mutableStateOf(
            initialTouchBridgeDevices(
                usbAttachmentState = connectionManager.usbAttachmentState(),
            ),
        )
    }
    var screen by remember { mutableStateOf(AppScreen.DeviceList) }
    var selectedDeviceId by rememberSaveable { mutableStateOf<String?>(null) }
    var pairingDeviceId by rememberSaveable { mutableStateOf<String?>(null) }
    var settingsDeviceId by rememberSaveable { mutableStateOf<String?>(null) }
    var appSettingsVisible by rememberSaveable { mutableStateOf(false) }
    var gestureVibrationEnabled by rememberSaveable {
        mutableStateOf(loadGestureVibrationEnabled(context))
    }
    var customButtonEditorVisible by rememberSaveable { mutableStateOf(false) }
    var customButtons by remember {
        mutableStateOf(loadCustomButtons(context))
    }

    var gestureStatus by rememberSaveable { mutableStateOf(GestureSendStatus.Idle) }
    var lastGesture by remember { mutableStateOf<TouchBridgeGesture?>(null) }
    var gestureErrorMessage by rememberSaveable { mutableStateOf<String?>(null) }
    var isSending by rememberSaveable { mutableStateOf(false) }

    fun refreshUsbAvailability() {
        val usbAttachmentState = connectionManager.usbAttachmentState()
        devices = devices.map { device ->
            if (device.transport != TransportType.Usb) {
                device
            } else {
                device.copy(
                    available = usbAttachmentState != UsbAttachmentState.CableDisconnected,
                    usbAttachmentState = usbAttachmentState,
                    connectionStatus = if (usbAttachmentState == UsbAttachmentState.CableDisconnected &&
                        device.connectionStatus == ConnectionStatus.Connected
                    ) {
                        ConnectionStatus.Disconnected
                    } else {
                        device.connectionStatus
                    },
                )
            }
        }
    }

    fun showMessage(message: String) {
        scope.launch {
            snackbarHostState.showSnackbar(message)
        }
    }

    fun updateGestureVibrationEnabled(enabled: Boolean) {
        gestureVibrationEnabled = enabled
        saveGestureVibrationEnabled(context, enabled)
    }

    fun updateCustomButtons(nextButtons: List<CustomButton>) {
        val normalized = nextButtons.normalizedCustomButtonPositions()
        customButtons = normalized
        saveCustomButtons(context, normalized)

        if (selectedDeviceId != null) {
            scope.launch {
                connectionManager.syncCustomButtons(normalized).onFailure { throwable ->
                    val activeDevice = devices.firstOrNull { it.id == selectedDeviceId }
                    showMessage(connectionErrorMessage(context, activeDevice, throwable))
                }
            }
        }
    }

    fun addCustomButton(id: String, label: String) {
        val sanitizedId = sanitizeCustomButtonId(id)
        if (sanitizedId.isBlank() || label.isBlank()) {
            showMessage(context.getString(R.string.error_custom_button_required))
            return
        }

        if (customButtons.any { it.id == sanitizedId }) {
            showMessage(context.getString(R.string.error_custom_button_duplicate))
            return
        }

        updateCustomButtons(
            customButtons + CustomButton(
                id = sanitizedId,
                label = label.trim(),
                position = customButtons.size,
            ),
        )
        customButtonEditorVisible = false
    }

    fun moveCustomButton(button: CustomButton, direction: Int) {
        val ordered = customButtons.normalizedCustomButtonPositions().toMutableList()
        val fromIndex = ordered.indexOfFirst { it.id == button.id }
        val toIndex = (fromIndex + direction).coerceIn(0, ordered.lastIndex)

        if (fromIndex < 0 || fromIndex == toIndex) {
            return
        }

        val item = ordered.removeAt(fromIndex)
        ordered.add(toIndex, item)
        updateCustomButtons(ordered)
    }

    fun deleteCustomButton(button: CustomButton) {
        updateCustomButtons(customButtons.filterNot { it.id == button.id })
    }

    fun scanForBleDevices() {
        if (!permissionsGranted) {
            onRequestPermissions()
            showMessage(context.getString(R.string.error_bluetooth_permission_required))
            return
        }

        showMessage(context.getString(R.string.ble_scan_started))
        scope.launch {
            connectionManager.scanBleDevices().fold(
                onSuccess = { scannedDevices ->
                    if (scannedDevices.isEmpty()) {
                        showMessage(context.getString(R.string.ble_scan_no_devices))
                        return@fold
                    }

                    devices = (devices.filter { existing ->
                        scannedDevices.none { scanned -> scanned.id == existing.id }
                    } + scannedDevices).sortedWith(
                        compareBy<Device> { it.transport != TransportType.Usb }
                            .thenBy { it.name },
                    )
                    showMessage(context.getString(R.string.ble_scan_found_format, scannedDevices.size))
                },
                onFailure = { throwable ->
                    showMessage(
                        throwable.message ?: context.getString(R.string.error_ble_scan_failed),
                    )
                },
            )
        }
    }

    DisposableEffect(connectionManager) {
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(receiverContext: Context, intent: Intent) {
                when (intent.action) {
                    ACTION_USB_STATE,
                    UsbManager.ACTION_USB_ACCESSORY_ATTACHED,
                    UsbManager.ACTION_USB_ACCESSORY_DETACHED,
                    -> refreshUsbAvailability()
                }
            }
        }
        val filter = IntentFilter().apply {
            addAction(ACTION_USB_STATE)
            addAction(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)
            addAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED)
        }
        ContextCompat.registerReceiver(
            context,
            receiver,
            filter,
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
        refreshUsbAvailability()

        onDispose {
            runCatching { context.unregisterReceiver(receiver) }
        }
    }

    fun updateDevice(deviceId: String, transform: (Device) -> Device) {
        devices = devices.map { device ->
            if (device.id == deviceId) transform(device) else device
        }
    }

    fun startConnection(deviceId: String) {
        val device = devices.firstOrNull { it.id == deviceId } ?: return

        if (device.transport == TransportType.Ble && !permissionsGranted) {
            onRequestPermissions()
            showMessage(context.getString(R.string.error_bluetooth_permission_required))
            return
        }

        if (device.transport == TransportType.Ble && !device.available) {
            updateDevice(deviceId) {
                it.copy(connectionStatus = ConnectionStatus.Failed)
            }
            showMessage(context.getString(R.string.error_device_unavailable_format, device.name))
            return
        }

        devices = devices.map { item ->
            when (item.id) {
                deviceId -> item.copy(connectionStatus = ConnectionStatus.Connecting)
                else -> item.copy(
                    connectionStatus = if (item.connectionStatus == ConnectionStatus.Connected) {
                        ConnectionStatus.Disconnected
                    } else {
                        item.connectionStatus
                    },
                )
            }
        }

        scope.launch {
            val result = connectionManager.connectToDevice(device)

            result.fold(
                onSuccess = {
                    connectionManager.syncCustomButtons(customButtons).onFailure { throwable ->
                        showMessage(connectionErrorMessage(context, device, throwable))
                    }
                    devices = devices.map { item ->
                        when (item.id) {
                            deviceId -> item.copy(
                                paired = true,
                                trusted = true,
                                connectionStatus = ConnectionStatus.Connected,
                                lastConnectedAt = System.currentTimeMillis(),
                            )

                            else -> item.copy(
                                connectionStatus = if (item.connectionStatus == ConnectionStatus.Connected) {
                                    ConnectionStatus.Disconnected
                                } else {
                                    item.connectionStatus
                                },
                            )
                        }
                    }
                    selectedDeviceId = deviceId
                    pairingDeviceId = null
                    gestureStatus = GestureSendStatus.Idle
                    gestureErrorMessage = null
                    screen = AppScreen.GesturePad
                },
                onFailure = { throwable ->
                    updateDevice(deviceId) {
                        it.copy(connectionStatus = ConnectionStatus.Failed)
                    }
                    showMessage(connectionErrorMessage(context, device, throwable))
                }
            )
        }
    }

    fun handleDeviceClick(device: Device) {
        if (device.capabilities.requiresPairing && (!device.paired || !device.trusted)) {
            pairingDeviceId = device.id
            screen = AppScreen.Pairing
        } else {
            startConnection(device.id)
        }
    }

    fun sendGesture(gesture: TouchBridgeGesture) {
        val activeDeviceId = selectedDeviceId
        if (activeDeviceId == null) {
            gestureStatus = GestureSendStatus.Error
            gestureErrorMessage = context.getString(R.string.error_no_active_connection)
            return
        }

        val activeDevice = devices.firstOrNull { it.id == activeDeviceId }
        if (activeDevice?.transport == TransportType.Ble && !permissionsGranted) {
            gestureStatus = GestureSendStatus.Error
            gestureErrorMessage = context.getString(R.string.error_bluetooth_permission_required)
            onRequestPermissions()
            return
        }

        lastGesture = gesture
        gestureStatus = GestureSendStatus.Sending
        gestureErrorMessage = null
        isSending = true

        scope.launch {
            val result = connectionManager.sendGestureEvent(
                GestureEvent(
                    deviceId = activeDeviceId,
                    gesture = gesture,
                ),
            )
            isSending = false

            result.fold(
                onSuccess = {
                    gestureStatus = GestureSendStatus.Sent
                    gestureErrorMessage = null
                    if (gestureVibrationEnabled) {
                        vibrateForGesture(context)
                    }
                },
                onFailure = { throwable ->
                    gestureStatus = GestureSendStatus.Error
                    gestureErrorMessage = connectionErrorMessage(context, activeDevice, throwable)
                },
            )
        }
    }

    fun sendCustomButton(button: CustomButton) {
        if (selectedDeviceId == null) {
            showMessage(context.getString(R.string.error_no_active_connection))
            return
        }

        isSending = true
        scope.launch {
            val activeDevice = devices.firstOrNull { it.id == selectedDeviceId }
            val result = connectionManager.sendCustomButtonEvent(button.id)
            isSending = false

            result.fold(
                onSuccess = {
                    if (gestureVibrationEnabled) {
                        vibrateForGesture(context)
                    }
                },
                onFailure = { throwable ->
                    showMessage(connectionErrorMessage(context, activeDevice, throwable))
                },
            )
        }
    }

    val selectedDevice = devices.firstOrNull { it.id == selectedDeviceId }
    val pairingDevice = devices.firstOrNull { it.id == pairingDeviceId }
    val settingsDevice = devices.firstOrNull { it.id == settingsDeviceId }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        containerColor = AppBackground,
        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.verticalGradient(
                        colors = listOf(
                            Color(0xFF07111D),
                            AppBackground,
                            Color(0xFF07090E),
                        ),
                    ),
                )
                .padding(innerPadding)
                .padding(WindowInsets.safeDrawing.asPaddingValues()),
        ) {
            when (screen) {
                AppScreen.DeviceList -> DeviceListScreen(
                    devices = devices,
                    permissionsGranted = permissionsGranted,
                    onRequestPermissions = onRequestPermissions,
                    onDeviceClick = ::handleDeviceClick,
                    onDeviceOptionsClick = { settingsDeviceId = it.id },
                    onNewDeviceClick = ::scanForBleDevices,
                    onAddCustomButtonClick = { customButtonEditorVisible = true },
                    onAppSettingsClick = { appSettingsVisible = true },
                )

                AppScreen.Pairing -> {
                    if (pairingDevice == null) {
                        DeviceListScreen(
                            devices = devices,
                            permissionsGranted = permissionsGranted,
                            onRequestPermissions = onRequestPermissions,
                            onDeviceClick = ::handleDeviceClick,
                            onDeviceOptionsClick = { settingsDeviceId = it.id },
                            onNewDeviceClick = ::scanForBleDevices,
                            onAddCustomButtonClick = { customButtonEditorVisible = true },
                            onAppSettingsClick = { appSettingsVisible = true },
                        )
                    } else {
                        PairingScreen(
                            device = pairingDevice,
                            permissionsGranted = permissionsGranted,
                            onRequestPermissions = onRequestPermissions,
                            onAllow = {
                                if (!permissionsGranted) {
                                    onRequestPermissions()
                                    showMessage(context.getString(R.string.error_bluetooth_permission_required))
                                } else {
                                    updateDevice(pairingDevice.id) {
                                        it.copy(paired = true, trusted = true)
                                    }
                                    startConnection(pairingDevice.id)
                                }
                            },
                            onDeny = {
                                pairingDeviceId = null
                                screen = AppScreen.DeviceList
                            },
                        )
                    }
                }

                AppScreen.GesturePad -> {
                    if (selectedDevice == null) {
                        DeviceListScreen(
                            devices = devices,
                            permissionsGranted = permissionsGranted,
                            onRequestPermissions = onRequestPermissions,
                            onDeviceClick = ::handleDeviceClick,
                            onDeviceOptionsClick = { settingsDeviceId = it.id },
                            onNewDeviceClick = ::scanForBleDevices,
                            onAddCustomButtonClick = { customButtonEditorVisible = true },
                            onAppSettingsClick = { appSettingsVisible = true },
                        )
                    } else {
                        GesturePadScreen(
                            device = selectedDevice,
                            customButtons = customButtons,
                            gestureStatus = gestureStatus,
                            lastGesture = lastGesture,
                            gestureErrorMessage = gestureErrorMessage,
                            isSending = isSending,
                            onGesture = ::sendGesture,
                            onCustomButton = ::sendCustomButton,
                            onAddCustomButtonClick = { customButtonEditorVisible = true },
                            onMoveCustomButton = ::moveCustomButton,
                            onDeleteCustomButton = ::deleteCustomButton,
                            onBack = { screen = AppScreen.DeviceList },
                            onAppSettingsClick = { appSettingsVisible = true },
                        )
                    }
                }
            }
        }

        if (settingsDevice != null) {
            DeviceOptionsSheet(
                device = settingsDevice,
                onDismiss = { settingsDeviceId = null },
                onRename = { newName ->
                    updateDevice(settingsDevice.id) {
                        it.copy(name = newName)
                    }
                },
                onAutoConnectChange = { enabled ->
                    updateDevice(settingsDevice.id) {
                        it.copy(autoConnect = enabled)
                    }
                },
                onDisconnect = {
                    updateDevice(settingsDevice.id) {
                        it.copy(connectionStatus = ConnectionStatus.Disconnected)
                    }
                    if (selectedDeviceId == settingsDevice.id) {
                        connectionManager.disconnect()
                        selectedDeviceId = null
                        screen = AppScreen.DeviceList
                    }
                    settingsDeviceId = null
                },
                onRemoveTrust = {
                    updateDevice(settingsDevice.id) {
                        it.copy(
                            paired = false,
                            trusted = false,
                            autoConnect = false,
                            connectionStatus = ConnectionStatus.Disconnected,
                        )
                    }
                    if (selectedDeviceId == settingsDevice.id) {
                        connectionManager.disconnect()
                        selectedDeviceId = null
                        screen = AppScreen.DeviceList
                    }
                    settingsDeviceId = null
                },
            )
        }

        if (appSettingsVisible) {
            AppSettingsSheet(
                gestureVibrationEnabled = gestureVibrationEnabled,
                onGestureVibrationChange = ::updateGestureVibrationEnabled,
                appLanguage = appLanguage,
                onAppLanguageChange = onAppLanguageChange,
                onAddCustomButtonClick = {
                    appSettingsVisible = false
                    customButtonEditorVisible = true
                },
                onDismiss = { appSettingsVisible = false },
            )
        }

        if (customButtonEditorVisible) {
            CustomButtonEditorSheet(
                onDismiss = { customButtonEditorVisible = false },
                onAdd = ::addCustomButton,
            )
        }
    }
}

@Composable
private fun DeviceListScreen(
    devices: List<Device>,
    permissionsGranted: Boolean,
    onRequestPermissions: () -> Unit,
    onDeviceClick: (Device) -> Unit,
    onDeviceOptionsClick: (Device) -> Unit,
    onNewDeviceClick: () -> Unit,
    onAddCustomButtonClick: () -> Unit,
    onAppSettingsClick: () -> Unit,
) {
    val usbDevices = devices.filter { it.transport == TransportType.Usb }
    val pairedDevices = devices.filter {
        it.transport == TransportType.Ble && (it.paired || it.trusted)
    }
    val nearbyDevices = devices.filter {
        it.transport == TransportType.Ble && !it.paired && !it.trusted
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 18.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        item {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = stringResource(R.string.app_name),
                        style = MaterialTheme.typography.headlineLarge,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onBackground,
                    )
                    TextButton(onClick = onAppSettingsClick) {
                        Text(text = stringResource(R.string.app_settings))
                    }
                }
                Text(
                    text = stringResource(R.string.device_list_subtitle),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        if (!permissionsGranted) {
            item {
                PermissionBanner(onRequestPermissions = onRequestPermissions)
            }
        }

        item {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Button(
                    onClick = onNewDeviceClick,
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = 54.dp),
                    shape = RoundedCornerShape(8.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AppBlue),
                ) {
                    Text(
                        text = stringResource(R.string.new_device_connect),
                        color = Color(0xFF07111D),
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                FilledTonalButton(
                    onClick = onAddCustomButtonClick,
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = 52.dp),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(
                        text = stringResource(R.string.add_custom_button),
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }

        item {
            DeviceListSection(
                title = stringResource(R.string.usb_devices_section),
                emptyText = stringResource(R.string.empty_usb_devices),
                devices = usbDevices,
                onDeviceClick = onDeviceClick,
                onDeviceOptionsClick = onDeviceOptionsClick,
            )
        }

        item {
            DeviceListSection(
                title = stringResource(R.string.paired_devices_section),
                emptyText = stringResource(R.string.empty_paired_devices),
                devices = pairedDevices,
                onDeviceClick = onDeviceClick,
                onDeviceOptionsClick = onDeviceOptionsClick,
            )
        }

        item {
            DeviceListSection(
                title = stringResource(R.string.nearby_devices_section),
                emptyText = stringResource(R.string.empty_nearby_devices),
                devices = nearbyDevices,
                onDeviceClick = onDeviceClick,
                onDeviceOptionsClick = onDeviceOptionsClick,
            )
        }
    }
}

@Composable
private fun PermissionBanner(onRequestPermissions: () -> Unit) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
        border = BorderStroke(1.dp, AppAmber.copy(alpha = 0.38f)),
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                Text(
                    text = stringResource(R.string.bluetooth_permission_title),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = stringResource(R.string.bluetooth_permission_body),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            FilledTonalButton(
                onClick = onRequestPermissions,
                shape = RoundedCornerShape(8.dp),
            ) {
                Text(stringResource(R.string.allow))
            }
        }
    }
}

@Composable
private fun DeviceListSection(
    title: String,
    emptyText: String,
    devices: List<Device>,
    onDeviceClick: (Device) -> Unit,
    onDeviceOptionsClick: (Device) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onBackground,
        )

        if (devices.isEmpty()) {
            EmptyDeviceCard(text = emptyText)
        } else {
            devices.forEach { device ->
                DeviceCard(
                    device = device,
                    onClick = { onDeviceClick(device) },
                    onOptionsClick = { onDeviceOptionsClick(device) },
                )
            }
        }
    }
}

@Composable
private fun EmptyDeviceCard(text: String) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = AppSurface,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        Text(
            text = text,
            modifier = Modifier.padding(18.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun DeviceCard(
    device: Device,
    onClick: () -> Unit,
    onOptionsClick: () -> Unit,
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 104.dp)
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
        border = BorderStroke(
            1.dp,
            if (device.connectionStatus == ConnectionStatus.Connected) {
                AppGreen.copy(alpha = 0.52f)
            } else {
                AppOutline
            },
        ),
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            StatusDot(
                status = device.connectionStatus,
                available = device.available,
                modifier = Modifier.align(Alignment.Top),
            )

            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    text = device.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = "${transportLabel(device.transport)} · ${deviceOsLabel(device.os)} · ${deviceTypeLabel(device.type)}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = deviceStatusText(device),
                    style = MaterialTheme.typography.labelLarge,
                    color = deviceStatusColor(device),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }

            IconButton(onClick = onOptionsClick) {
                Text(
                    text = "⋯",
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun StatusDot(
    status: ConnectionStatus,
    available: Boolean,
    modifier: Modifier = Modifier,
) {
    if (status == ConnectionStatus.Connecting) {
        CircularProgressIndicator(
            modifier = modifier.size(18.dp),
            strokeWidth = 2.dp,
            color = AppAmber,
        )
        return
    }

    val color = when (status) {
        ConnectionStatus.Connected -> AppGreen
        ConnectionStatus.Failed -> AppRed
        ConnectionStatus.Connecting -> AppAmber
        ConnectionStatus.Disconnected -> if (available) AppGreen else Color(0xFF5F6D7C)
    }

    Box(
        modifier = modifier
            .padding(top = 6.dp)
            .size(11.dp)
            .clip(CircleShape)
            .background(color),
    )
}

@Composable
private fun PairingScreen(
    device: Device,
    permissionsGranted: Boolean,
    onRequestPermissions: () -> Unit,
    onAllow: () -> Unit,
    onDeny: () -> Unit,
) {
    BackHandler(onBack = onDeny)

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 18.dp),
        verticalArrangement = Arrangement.spacedBy(18.dp),
    ) {
        TextButton(onClick = onDeny) {
            Text(text = stringResource(R.string.back_to_devices))
        }

        PairingPermissionCard(
            device = device,
            permissionsGranted = permissionsGranted,
            onRequestPermissions = onRequestPermissions,
            onAllow = onAllow,
            onDeny = onDeny,
            modifier = Modifier.weight(1f),
        )
    }
}

@Composable
private fun PairingPermissionCard(
    device: Device,
    permissionsGranted: Boolean,
    onRequestPermissions: () -> Unit,
    onAllow: () -> Unit,
    onDeny: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        Column(
            modifier = Modifier.padding(22.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                Text(
                    text = stringResource(R.string.pairing_title),
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(
                        text = device.name,
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                        color = AppBlue,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "${deviceOsLabel(device.os)} · ${deviceTypeLabel(device.type)}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text(
                    text = stringResource(R.string.pairing_body),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                if (!permissionsGranted) {
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = AppAmber.copy(alpha = 0.12f),
                        border = BorderStroke(1.dp, AppAmber.copy(alpha = 0.35f)),
                    ) {
                        Row(
                            modifier = Modifier.padding(12.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = stringResource(R.string.bluetooth_permission_body),
                                modifier = Modifier.weight(1f),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            TextButton(onClick = onRequestPermissions) {
                                Text(stringResource(R.string.allow))
                            }
                        }
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                OutlinedButton(
                    onClick = onDeny,
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(stringResource(R.string.cancel))
                }
                Button(
                    onClick = onAllow,
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(8.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AppGreen),
                ) {
                    Text(
                        text = stringResource(R.string.allow),
                        color = Color(0xFF06110C),
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
    }
}

@Composable
private fun GesturePadScreen(
    device: Device,
    customButtons: List<CustomButton>,
    gestureStatus: GestureSendStatus,
    lastGesture: TouchBridgeGesture?,
    gestureErrorMessage: String?,
    isSending: Boolean,
    onGesture: (TouchBridgeGesture) -> Unit,
    onCustomButton: (CustomButton) -> Unit,
    onAddCustomButtonClick: () -> Unit,
    onMoveCustomButton: (CustomButton, Int) -> Unit,
    onDeleteCustomButton: (CustomButton) -> Unit,
    onBack: () -> Unit,
    onAppSettingsClick: () -> Unit,
) {
    BackHandler(onBack = onBack)
    var padMode by rememberSaveable { mutableStateOf(GesturePadMode.TouchPad) }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 14.dp),
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            GesturePadHeader(
                device = device,
                mode = padMode,
                onBack = onBack,
                onAppSettingsClick = onAppSettingsClick,
            )

            if (padMode == GesturePadMode.TouchPad) {
                GesturePad(
                    statusText = gestureStatusText(
                        status = gestureStatus,
                        lastGesture = lastGesture,
                        errorMessage = gestureErrorMessage,
                    ),
                    status = gestureStatus,
                    lastGestureLabel = lastGesture?.let { gestureLabel(it) } ?: stringResource(R.string.none),
                    isSending = isSending,
                    onGesture = onGesture,
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f),
                )
                GesturePadFooter(lastGesture = lastGesture)
                QuickGestureBar(onGesture = onGesture)
            } else {
                CustomButtonGrid(
                    buttons = customButtons,
                    isSending = isSending,
                    onButtonClick = onCustomButton,
                    onAddClick = onAddCustomButtonClick,
                    onMove = onMoveCustomButton,
                    onDelete = onDeleteCustomButton,
                    modifier = Modifier
                        .fillMaxWidth()
                        .weight(1f),
                )
            }
        }

        FloatingActionButton(
            onClick = {
                padMode = if (padMode == GesturePadMode.TouchPad) {
                    GesturePadMode.CustomButtons
                } else {
                    GesturePadMode.TouchPad
                }
            },
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(bottom = 10.dp),
            containerColor = AppBlue,
            contentColor = Color(0xFF07111D),
        ) {
            Text(
                text = if (padMode == GesturePadMode.TouchPad) {
                    stringResource(R.string.custom_buttons_short)
                } else {
                    stringResource(R.string.touch_pad_short)
                },
                fontWeight = FontWeight.Bold,
            )
        }

        FloatingActionButton(
            onClick = onAddCustomButtonClick,
            modifier = Modifier
                .align(Alignment.BottomStart)
                .padding(bottom = 10.dp),
            containerColor = AppGreen,
            contentColor = Color(0xFF06110C),
        ) {
            Text(
                text = "+",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}

@Composable
private fun GesturePadHeader(
    device: Device,
    mode: GesturePadMode,
    onBack: () -> Unit,
    onAppSettingsClick: () -> Unit,
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(onClick = onBack) {
                Text(text = "‹")
            }
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                Text(
                    text = device.name,
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = "${connectedTransportLabel(device.transport)} · ${gesturePadModeLabel(mode)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Row(
                horizontalArrangement = Arrangement.spacedBy(7.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                StatusDot(status = device.connectionStatus, available = device.available)
                Text(
                    text = connectionStatusLabel(device.connectionStatus),
                    style = MaterialTheme.typography.labelMedium,
                    color = deviceStatusColor(device),
                )
                TextButton(onClick = onAppSettingsClick) {
                    Text(text = stringResource(R.string.app_settings))
                }
            }
        }
    }
}

@Composable
private fun GesturePadFooter(lastGesture: TouchBridgeGesture?) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = AppSurface,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = stringResource(R.string.current_profile),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = stringResource(R.string.default_profile),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = stringResource(R.string.last_executed_gesture),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = lastGesture?.let { gestureLabel(it) } ?: stringResource(R.string.none),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DeviceOptionsSheet(
    device: Device,
    onDismiss: () -> Unit,
    onRename: (String) -> Unit,
    onAutoConnectChange: (Boolean) -> Unit,
    onDisconnect: () -> Unit,
    onRemoveTrust: () -> Unit,
) {
    var draftName by remember(device.id) { mutableStateOf(device.name) }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = AppSurfaceHigh,
        contentColor = MaterialTheme.colorScheme.onSurface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 18.dp)
                .padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                text = stringResource(R.string.device_settings),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            OutlinedTextField(
                value = draftName,
                onValueChange = { draftName = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.device_name_label)) },
            )
            Button(
                onClick = {
                    if (draftName.isNotBlank()) {
                        onRename(draftName.trim())
                    }
                },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(8.dp),
            ) {
                Text(stringResource(R.string.save_name))
            }

            HorizontalDivider(color = AppOutline)

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(
                        text = stringResource(R.string.auto_connect),
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = stringResource(R.string.auto_connect_body),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = device.autoConnect,
                    onCheckedChange = onAutoConnectChange,
                )
            }

            HorizontalDivider(color = AppOutline)

            OutlinedButton(
                onClick = onDisconnect,
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(8.dp),
            ) {
                Text(stringResource(R.string.disconnect))
            }
            TextButton(
                onClick = onRemoveTrust,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = stringResource(R.string.remove_permission),
                    color = AppRed,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AppSettingsSheet(
    gestureVibrationEnabled: Boolean,
    onGestureVibrationChange: (Boolean) -> Unit,
    appLanguage: AppLanguageSetting,
    onAppLanguageChange: (AppLanguageSetting) -> Unit,
    onAddCustomButtonClick: () -> Unit,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = AppSurfaceHigh,
        contentColor = MaterialTheme.colorScheme.onSurface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 18.dp)
                .padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                text = stringResource(R.string.app_settings),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(3.dp),
                ) {
                    Text(
                        text = stringResource(R.string.gesture_vibration),
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = stringResource(R.string.gesture_vibration_body),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = gestureVibrationEnabled,
                    onCheckedChange = onGestureVibrationChange,
                )
            }

            HorizontalDivider(color = AppOutline)

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = stringResource(R.string.app_language),
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold,
                )
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    AppLanguageSetting.values().forEach { language ->
                        LanguageChoiceButton(
                            language = language,
                            selected = language == appLanguage,
                            onClick = { onAppLanguageChange(language) },
                        )
                    }
                }
            }

            HorizontalDivider(color = AppOutline)

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = stringResource(R.string.custom_buttons),
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = stringResource(R.string.custom_buttons_body),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Button(
                    onClick = onAddCustomButtonClick,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(8.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AppBlue),
                ) {
                    Text(
                        text = stringResource(R.string.add_custom_button),
                        color = Color(0xFF07111D),
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
    }
}

@Composable
private fun LanguageChoiceButton(
    language: AppLanguageSetting,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val label = when (language) {
        AppLanguageSetting.System -> stringResource(R.string.app_language_system)
        AppLanguageSetting.English -> stringResource(R.string.app_language_english)
        AppLanguageSetting.Korean -> stringResource(R.string.app_language_korean)
    }

    if (selected) {
        Button(
            onClick = onClick,
            shape = RoundedCornerShape(8.dp),
            colors = ButtonDefaults.buttonColors(containerColor = AppBlue),
        ) {
            Text(
                text = label,
                color = Color(0xFF07111D),
                fontWeight = FontWeight.SemiBold,
            )
        }
    } else {
        OutlinedButton(
            onClick = onClick,
            shape = RoundedCornerShape(8.dp),
        ) {
            Text(text = label)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CustomButtonEditorSheet(
    onDismiss: () -> Unit,
    onAdd: (String, String) -> Unit,
) {
    var signalId by rememberSaveable { mutableStateOf("") }
    var buttonName by rememberSaveable { mutableStateOf("") }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = AppSurfaceHigh,
        contentColor = MaterialTheme.colorScheme.onSurface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 18.dp)
                .padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text(
                text = stringResource(R.string.add_custom_button),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            OutlinedTextField(
                value = signalId,
                onValueChange = { signalId = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.custom_button_signal_id)) },
            )
            OutlinedTextField(
                value = buttonName,
                onValueChange = { buttonName = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.custom_button_display_name)) },
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                OutlinedButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(stringResource(R.string.deny))
                }
                Button(
                    onClick = { onAdd(signalId, buttonName) },
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(stringResource(R.string.add_custom_button))
                }
            }
        }
    }
}

@Composable
private fun GesturePad(
    statusText: String,
    status: GestureSendStatus,
    lastGestureLabel: String,
    isSending: Boolean,
    onGesture: (TouchBridgeGesture) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scope = rememberCoroutineScope()
    var lastTapAt by remember { mutableStateOf(0L) }
    var lastTapPosition by remember { mutableStateOf(Offset.Zero) }
    var pendingTapJob by remember { mutableStateOf<Job?>(null) }

    val borderColor = when (status) {
        GestureSendStatus.Idle -> AppOutline
        GestureSendStatus.Sending -> AppAmber
        GestureSendStatus.Sent -> AppGreen
        GestureSendStatus.Error -> AppRed
    }

    Surface(
        modifier = modifier
            .heightIn(min = 320.dp)
            .border(2.dp, borderColor, RoundedCornerShape(8.dp))
            .pointerInput(onGesture) {
                awaitEachGesture {
                    val firstDown = awaitFirstDown(requireUnconsumed = false)
                    val longPressTimeout = viewConfiguration.longPressTimeoutMillis
                    val doubleTapTimeout = viewConfiguration.doubleTapTimeoutMillis
                    val touchSlop = viewConfiguration.touchSlop
                    val minSwipeDistance = touchSlop * 8f
                    val tapSlop = touchSlop * 2.2f

                    val pointerStarts = mutableMapOf(firstDown.id to firstDown.position)
                    var maxPointerCount = 1
                    var maxPointerTravel = 0f
                    var startCentroid = firstDown.position
                    var currentCentroid = firstDown.position
                    var upTime = firstDown.uptimeMillis

                    do {
                        val event = awaitPointerEvent()
                        val pressed = event.changes.filter { it.pressed }

                        pressed.forEach { change ->
                            val start = pointerStarts.getOrPut(change.id) { change.position }
                            maxPointerTravel = maxOf(
                                maxPointerTravel,
                                (change.position - start).getDistance(),
                            )
                        }

                        if (pressed.isNotEmpty()) {
                            val activeCentroid = pressed.centroid()

                            if (pressed.size > maxPointerCount) {
                                maxPointerCount = pressed.size
                                startCentroid = activeCentroid
                                currentCentroid = activeCentroid
                            } else if (pressed.size == maxPointerCount) {
                                currentCentroid = activeCentroid
                            }
                        }

                        upTime = event.changes.maxOf { it.uptimeMillis }
                    } while (event.changes.any { it.pressed })

                    val duration = upTime - firstDown.uptimeMillis
                    val trace = GestureTrace(
                        maxPointerCount = maxPointerCount,
                        durationMillis = duration,
                        centroidDelta = currentCentroid - startCentroid,
                        maxPointerTravel = maxPointerTravel,
                        completedPosition = currentCentroid,
                    )
                    val gesture = classifyGesture(
                        trace = trace,
                        minSwipeDistance = minSwipeDistance,
                        tapSlop = tapSlop,
                        longPressTimeout = longPressTimeout,
                    )

                    if (
                        gesture == null &&
                        trace.maxPointerCount == 1 &&
                        trace.maxPointerTravel <= tapSlop
                    ) {
                        val isDoubleTap = lastTapAt > 0 &&
                            upTime - lastTapAt <= doubleTapTimeout &&
                            (trace.completedPosition - lastTapPosition).getDistance() <= minSwipeDistance

                        if (isDoubleTap) {
                            pendingTapJob?.cancel()
                            pendingTapJob = null
                            lastTapAt = 0L
                            onGesture(TouchBridgeGesture.DoubleTap)
                        } else {
                            pendingTapJob?.cancel()
                            lastTapAt = upTime
                            lastTapPosition = trace.completedPosition
                            pendingTapJob = scope.launch {
                                delay(doubleTapTimeout)
                                lastTapAt = 0L
                                pendingTapJob = null
                                onGesture(TouchBridgeGesture.Tap)
                            }
                        }
                    } else if (gesture != null) {
                        pendingTapJob?.cancel()
                        pendingTapJob = null
                        lastTapAt = 0L
                        onGesture(gesture)
                    } else {
                        pendingTapJob?.cancel()
                        pendingTapJob = null
                        lastTapAt = 0L
                    }
                }
            },
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.radialGradient(
                        colors = listOf(
                            AppBlue.copy(alpha = 0.22f),
                            AppGreen.copy(alpha = 0.08f),
                            Color.Transparent,
                        ),
                    ),
                )
                .padding(18.dp),
        ) {
            GesturePadBoundaryChrome(color = borderColor)

            Column(
                modifier = Modifier.align(Alignment.Center),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    text = lastGestureLabel,
                    style = MaterialTheme.typography.displaySmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    textAlign = TextAlign.Center,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = statusText,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }

            Text(
                text = stringResource(R.string.touch_pad),
                modifier = Modifier.align(Alignment.TopStart),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            if (isSending) {
                LinearProgressIndicator(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth(),
                    color = AppAmber,
                    trackColor = AppOutline,
                )
            }
        }
    }
}

@Composable
private fun QuickGestureBar(onGesture: (TouchBridgeGesture) -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        TouchBridgeGesture.entries.forEach { gesture ->
            OutlinedButton(
                onClick = { onGesture(gesture) },
                shape = RoundedCornerShape(8.dp),
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MaterialTheme.colorScheme.onSurface,
                ),
            ) {
                Text(
                    text = gestureShortLabel(gesture),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun CustomButtonGrid(
    buttons: List<CustomButton>,
    isSending: Boolean,
    onButtonClick: (CustomButton) -> Unit,
    onAddClick: () -> Unit,
    onMove: (CustomButton, Int) -> Unit,
    onDelete: (CustomButton) -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(8.dp),
        color = AppSurfaceHigh,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                        Text(
                            text = stringResource(R.string.custom_buttons),
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = stringResource(R.string.custom_buttons_body),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Button(
                        onClick = onAddClick,
                        shape = RoundedCornerShape(8.dp),
                    ) {
                        Text(stringResource(R.string.add_custom_button))
                    }
                }
            }

            if (buttons.isEmpty()) {
                item {
                    EmptyDeviceCard(text = stringResource(R.string.empty_custom_buttons))
                }
            } else {
                buttons.normalizedCustomButtonPositions().chunked(2).forEach { rowButtons ->
                    item {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            rowButtons.forEach { button ->
                                CustomButtonCard(
                                    button = button,
                                    isSending = isSending,
                                    onClick = { onButtonClick(button) },
                                    onMoveUp = { onMove(button, -1) },
                                    onMoveDown = { onMove(button, 1) },
                                    onDelete = { onDelete(button) },
                                    modifier = Modifier.weight(1f),
                                )
                            }

                            if (rowButtons.size == 1) {
                                Box(modifier = Modifier.weight(1f))
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CustomButtonCard(
    button: CustomButton,
    isSending: Boolean,
    onClick: () -> Unit,
    onMoveUp: () -> Unit,
    onMoveDown: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.heightIn(min = 132.dp),
        shape = RoundedCornerShape(8.dp),
        color = AppSurface,
        border = BorderStroke(1.dp, AppOutline),
    ) {
        Column(
            modifier = Modifier.padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Button(
                onClick = onClick,
                enabled = !isSending,
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 48.dp),
                shape = RoundedCornerShape(8.dp),
                colors = ButtonDefaults.buttonColors(containerColor = AppBlue),
            ) {
                Text(
                    text = button.label,
                    color = Color(0xFF07111D),
                    fontWeight = FontWeight.Bold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                text = button.id,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                TextButton(
                    onClick = onMoveUp,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.move_up))
                }
                TextButton(
                    onClick = onMoveDown,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.move_down))
                }
            }
            TextButton(
                onClick = onDelete,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = stringResource(R.string.delete_custom_button),
                    color = AppRed,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun GesturePadBoundaryChrome(color: Color) {
    Box(modifier = Modifier.fillMaxSize()) {
        BoundaryCorner(
            modifier = Modifier.align(Alignment.TopStart),
            color = color,
            horizontalAlignment = Alignment.TopStart,
            verticalAlignment = Alignment.TopStart,
        )
        BoundaryCorner(
            modifier = Modifier.align(Alignment.TopEnd),
            color = color,
            horizontalAlignment = Alignment.TopEnd,
            verticalAlignment = Alignment.TopEnd,
        )
        BoundaryCorner(
            modifier = Modifier.align(Alignment.BottomStart),
            color = color,
            horizontalAlignment = Alignment.BottomStart,
            verticalAlignment = Alignment.BottomStart,
        )
        BoundaryCorner(
            modifier = Modifier.align(Alignment.BottomEnd),
            color = color,
            horizontalAlignment = Alignment.BottomEnd,
            verticalAlignment = Alignment.BottomEnd,
        )
    }
}

@Composable
private fun BoundaryCorner(
    modifier: Modifier,
    color: Color,
    horizontalAlignment: Alignment,
    verticalAlignment: Alignment,
) {
    Box(
        modifier = modifier.size(32.dp),
    ) {
        Box(
            modifier = Modifier
                .align(horizontalAlignment)
                .size(width = 28.dp, height = 3.dp)
                .background(color, RoundedCornerShape(2.dp)),
        )
        Box(
            modifier = Modifier
                .align(verticalAlignment)
                .size(width = 3.dp, height = 28.dp)
                .background(color, RoundedCornerShape(2.dp)),
        )
    }
}

@Composable
private fun deviceStatusText(device: Device): String {
    if (device.transport == TransportType.Usb) {
        return when (device.connectionStatus) {
            ConnectionStatus.Connected -> stringResource(R.string.status_usb_connected)
            ConnectionStatus.Connecting -> stringResource(R.string.status_connecting)
            ConnectionStatus.Failed -> stringResource(R.string.status_failed)
            ConnectionStatus.Disconnected -> when (device.usbAttachmentState) {
                UsbAttachmentState.AccessoryAvailable -> stringResource(R.string.status_usb_available)
                UsbAttachmentState.CableConnected -> stringResource(R.string.status_usb_cable_connected_agent_waiting)
                else -> stringResource(R.string.status_usb_connect_cable)
            }
        }
    }

    return when (device.connectionStatus) {
        ConnectionStatus.Connected -> stringResource(R.string.status_connected)
        ConnectionStatus.Connecting -> stringResource(R.string.status_connecting)
        ConnectionStatus.Failed -> stringResource(R.string.status_failed)
        ConnectionStatus.Disconnected -> when {
            device.available && !device.paired -> stringResource(R.string.status_available)
            device.lastConnectedAt != null -> stringResource(
                R.string.status_last_connected_format,
                formatTime(device.lastConnectedAt),
            )

            device.paired -> stringResource(R.string.status_paired)
            else -> stringResource(R.string.status_disconnected)
        }
    }
}

@Composable
private fun connectionStatusLabel(status: ConnectionStatus): String {
    return when (status) {
        ConnectionStatus.Disconnected -> stringResource(R.string.status_disconnected)
        ConnectionStatus.Connecting -> stringResource(R.string.status_connecting)
        ConnectionStatus.Connected -> stringResource(R.string.status_connected)
        ConnectionStatus.Failed -> stringResource(R.string.status_failed)
    }
}

@Composable
private fun gestureStatusText(
    status: GestureSendStatus,
    lastGesture: TouchBridgeGesture?,
    errorMessage: String?,
): String {
    val gestureName = lastGesture?.let { gestureLabel(it) } ?: stringResource(R.string.none)

    return when (status) {
        GestureSendStatus.Idle -> stringResource(R.string.gesture_idle)
        GestureSendStatus.Sending -> stringResource(
            R.string.gesture_sending_format,
            gestureName,
        )

        GestureSendStatus.Sent -> stringResource(
            R.string.gesture_sent_format,
            gestureName,
        )

        GestureSendStatus.Error -> stringResource(
            R.string.gesture_failed_format,
            errorMessage ?: stringResource(R.string.unknown_error),
        )
    }
}

@Composable
private fun gestureLabel(gesture: TouchBridgeGesture): String {
    return stringResource(gesture.labelRes)
}

@Composable
private fun gestureShortLabel(gesture: TouchBridgeGesture): String {
    return stringResource(gesture.shortLabelRes)
}

@Composable
private fun deviceTypeLabel(type: DeviceType): String {
    return when (type) {
        DeviceType.Laptop -> stringResource(R.string.device_type_laptop)
        DeviceType.Desktop -> stringResource(R.string.device_type_desktop)
    }
}

@Composable
private fun deviceOsLabel(os: DeviceOs): String {
    return when (os) {
        DeviceOs.Windows -> stringResource(R.string.device_os_windows_pc)
    }
}

@Composable
private fun transportLabel(transport: TransportType): String {
    return when (transport) {
        TransportType.Ble -> stringResource(R.string.transport_ble)
        TransportType.Usb -> stringResource(R.string.transport_usb)
    }
}

@Composable
private fun connectedTransportLabel(transport: TransportType): String {
    return when (transport) {
        TransportType.Ble -> stringResource(R.string.connected_via_ble)
        TransportType.Usb -> stringResource(R.string.connected_via_usb)
    }
}

@Composable
private fun gesturePadModeLabel(mode: GesturePadMode): String {
    return when (mode) {
        GesturePadMode.TouchPad -> stringResource(R.string.touch_pad)
        GesturePadMode.CustomButtons -> stringResource(R.string.custom_buttons)
    }
}

@Composable
private fun deviceStatusColor(device: Device): Color {
    return when (device.connectionStatus) {
        ConnectionStatus.Connected -> AppGreen
        ConnectionStatus.Connecting -> AppAmber
        ConnectionStatus.Failed -> AppRed
        ConnectionStatus.Disconnected -> when (device.usbAttachmentState) {
            UsbAttachmentState.AccessoryAvailable -> AppGreen
            UsbAttachmentState.CableConnected -> AppAmber
            else -> if (device.available) AppGreen else MaterialTheme.colorScheme.onSurfaceVariant
        }
    }
}

private fun formatTime(timestamp: Long): String {
    return SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date(timestamp))
}

private fun connectionErrorMessage(context: Context, device: Device?, throwable: Throwable): String {
    val message = throwable.message
    localizedConnectionErrorMessage(context, message)?.let { localizedMessage ->
        return localizedMessage
    }

    if (!message.isNullOrBlank()) {
        return message
    }

    return when (device?.transport) {
        TransportType.Usb -> context.getString(R.string.error_usb_connection_failed)
        TransportType.Ble -> context.getString(R.string.error_ble_connection_failed)
        null -> context.getString(R.string.error_connection_failed)
    }
}

private fun localizedConnectionErrorMessage(context: Context, message: String?): String? {
    if (message.isNullOrBlank()) {
        return null
    }

    return when {
        message == "No active TouchBridge connection" ->
            context.getString(R.string.error_no_active_connection)

        message == "Android Context is required" ->
            context.getString(R.string.unknown_error)

        message == "Bluetooth adapter is unavailable" ->
            context.getString(R.string.error_ble_adapter_unavailable)

        message == "Bluetooth is disabled" ->
            context.getString(R.string.error_ble_disabled)

        message == "BLE scanner is unavailable" ->
            context.getString(R.string.error_ble_scanner_unavailable)

        message == "TouchBridge BLE agent was not found" ->
            context.getString(R.string.error_ble_agent_not_found)

        message == "BLE characteristic write did not start" ->
            context.getString(R.string.error_ble_send_failed)

        message.startsWith("BLE characteristic write failed") ->
            context.getString(R.string.error_ble_send_failed)

        message.startsWith("BLE scan failed") ->
            context.getString(R.string.error_ble_scan_failed)

        message.startsWith("BLE connection timed out") ->
            context.getString(R.string.error_ble_timeout)

        message.startsWith("BLE connection failed") ||
            message.startsWith("BLE disconnected") ||
            message.startsWith("BLE service discovery") ->
            context.getString(R.string.error_ble_connection_failed)

        message.startsWith("Missing Bluetooth permission") ->
            context.getString(R.string.error_bluetooth_permission_required)

        else -> null
    }
}

private enum class AppLanguageSetting(
    val preferenceValue: String,
    val languageTag: String?,
) {
    System("system", null),
    English("en", "en"),
    Korean("ko", "ko");

    companion object {
        fun fromPreference(value: String?): AppLanguageSetting {
            return values().firstOrNull { it.preferenceValue == value } ?: System
        }
    }
}

private fun loadAppLanguageSetting(context: Context): AppLanguageSetting {
    val preference = context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .getString(PREF_APP_LANGUAGE, AppLanguageSetting.System.preferenceValue)

    return AppLanguageSetting.fromPreference(preference)
}

private fun saveAppLanguageSetting(context: Context, language: AppLanguageSetting) {
    context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putString(PREF_APP_LANGUAGE, language.preferenceValue)
        .apply()
}

private fun localizedContext(context: Context, language: AppLanguageSetting): Context {
    val languageTag = language.languageTag ?: return context
    val locale = Locale.forLanguageTag(languageTag)
    val configuration = Configuration(context.resources.configuration)

    configuration.setLocale(locale)
    configuration.setLocales(LocaleList(locale))

    return context.createConfigurationContext(configuration)
}

private fun applyPlatformAppLanguage(context: Context, language: AppLanguageSetting) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
        return
    }

    val locales = language.languageTag
        ?.let { LocaleList.forLanguageTags(it) }
        ?: LocaleList.getEmptyLocaleList()
    context.getSystemService(android.app.LocaleManager::class.java)
        ?.applicationLocales = locales
}

private fun loadGestureVibrationEnabled(context: Context): Boolean {
    return context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .getBoolean(PREF_GESTURE_VIBRATION_ENABLED, true)
}

private fun saveGestureVibrationEnabled(context: Context, enabled: Boolean) {
    context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putBoolean(PREF_GESTURE_VIBRATION_ENABLED, enabled)
        .apply()
}

@Suppress("DEPRECATION")
private fun vibrateForGesture(context: Context) {
    val vibrator = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        context.getSystemService(VibratorManager::class.java)?.defaultVibrator
    } else {
        context.getSystemService(Vibrator::class.java)
    } ?: return

    if (!vibrator.hasVibrator()) {
        return
    }

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        vibrator.vibrate(
            VibrationEffect.createOneShot(
                GESTURE_VIBRATION_MS,
                VibrationEffect.DEFAULT_AMPLITUDE,
            ),
        )
    } else {
        vibrator.vibrate(GESTURE_VIBRATION_MS)
    }
}

private fun classifyGesture(
    trace: GestureTrace,
    minSwipeDistance: Float,
    tapSlop: Float,
    longPressTimeout: Long,
): TouchBridgeGesture? {
    val delta = trace.centroidDelta
    val absX = abs(delta.x)
    val absY = abs(delta.y)
    val isTapMovement = trace.maxPointerTravel <= tapSlop
    val isShortEnoughForTap = trace.durationMillis < longPressTimeout
    val hasHorizontalIntent = absX >= minSwipeDistance && absX > absY * 1.25f
    val hasVerticalIntent = absY >= minSwipeDistance && absY > absX * 1.25f

    if (trace.maxPointerCount >= 3 && isShortEnoughForTap && isTapMovement) {
        return TouchBridgeGesture.ThreeFingerTap
    }

    if (trace.maxPointerCount == 2) {
        return when {
            hasHorizontalIntent -> {
                if (delta.x < 0f) TouchBridgeGesture.TwoFingerSwipeLeft else TouchBridgeGesture.TwoFingerSwipeRight
            }

            isShortEnoughForTap && isTapMovement -> TouchBridgeGesture.TwoFingerTap
            else -> null
        }
    }

    if (trace.maxPointerCount == 1 && trace.durationMillis >= longPressTimeout && isTapMovement) {
        return TouchBridgeGesture.LongPress
    }

    if (trace.maxPointerCount == 1 && (hasHorizontalIntent || hasVerticalIntent)) {
        return if (hasHorizontalIntent) {
            if (delta.x < 0f) TouchBridgeGesture.SwipeLeft else TouchBridgeGesture.SwipeRight
        } else {
            if (delta.y < 0f) TouchBridgeGesture.SwipeUp else TouchBridgeGesture.SwipeDown
        }
    }

    return null
}

private data class GestureTrace(
    val maxPointerCount: Int,
    val durationMillis: Long,
    val centroidDelta: Offset,
    val maxPointerTravel: Float,
    val completedPosition: Offset,
)

private fun List<PointerInputChange>.centroid(): Offset {
    if (isEmpty()) return Offset.Zero

    val x = sumOf { it.position.x.toDouble() }.toFloat() / size
    val y = sumOf { it.position.y.toDouble() }.toFloat() / size
    return Offset(x, y)
}

private enum class AppScreen {
    DeviceList,
    Pairing,
    GesturePad,
}

private enum class GestureSendStatus {
    Idle,
    Sending,
    Sent,
    Error,
}

private enum class GesturePadMode {
    TouchPad,
    CustomButtons,
}

@Preview(showBackground = true, backgroundColor = 0xFF050A12)
@Composable
private fun TouchBridgePreview() {
    TouchBridgeTheme(darkTheme = true) {
        TouchBridgeApp(
            client = object : TouchBridgeClient() {
                override suspend fun sendGesture(gesture: TouchBridgeGesture): Result<Unit> =
                    Result.success(Unit)
            },
        )
    }
}
