package com.sharon77770.touchbridge

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.content.ContextCompat
import java.io.BufferedReader
import java.io.BufferedWriter
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.Inet4Address
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.Socket
import java.net.SocketTimeoutException
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.json.JSONObject

private const val TAG = "UsbTransport"
private const val USB_PERMISSION_ACTION = "com.sharon77770.touchbridge.USB_PERMISSION"
private const val ACTION_USB_STATE = "android.hardware.usb.action.USB_STATE"
private const val EXTRA_USB_CONNECTED = "connected"
private const val USB_CONNECT_TIMEOUT_MS = 8_000L
private const val USB_ACCESSORY_DISCOVERY_TIMEOUT_MS = 4_000L
private const val USB_TCP_DISCOVERY_TIMEOUT_MS = 8_000L
private const val USB_TCP_BEACON_DISCOVERY_TIMEOUT_MS = 2_000L
private const val USB_TCP_PROBE_TIMEOUT_MS = 250
private const val USB_TCP_SCAN_BATCH_SIZE = 48
private const val USB_TCP_PORT = 47_831
private const val USB_TCP_BEACON_PORT = 47_832
private const val USB_TCP_BEACON_TYPE = "touchbridge_usb_tcp"
private const val USB_PERMISSION_TIMEOUT_MS = 15_000L
private const val ACCESSORY_MANUFACTURER = "TouchBridge"
private const val ACCESSORY_MODEL = "TouchBridge USB Agent"

class UsbTransport(
    private val context: Context,
) : ConnectionTransport {
    private val usbManager = context.getSystemService(UsbManager::class.java)
    private val connectivityManager = context.getSystemService(ConnectivityManager::class.java)
    private val mutex = Mutex()
    private var activeConnection: UsbConnection? = null

    fun isAccessoryAvailable(): Boolean {
        return findTouchBridgeAccessory() != null
    }

    fun attachmentState(): UsbAttachmentState {
        if (findTouchBridgeAccessory() != null) {
            return UsbAttachmentState.AccessoryAvailable
        }

        return if (isCableConnected()) {
            UsbAttachmentState.CableConnected
        } else {
            UsbAttachmentState.CableDisconnected
        }
    }

    override suspend fun connect(device: Device): Result<Unit> = mutex.withLock {
        runCatching {
            val connection = openBestUsbConnection()
            activeConnection = connection

            val ack = connection.exchange(handshakeMessage(device.id))
            if (!ack.ok) {
                close()
                throw UsbTransportException(
                    context.getString(
                        R.string.error_usb_handshake_failed_format,
                        ack.message.ifBlank {
                            context.getString(R.string.error_usb_invalid_response)
                        },
                    ),
                )
            }

            Log.d(TAG, "USB connected through ${connection.description}")
            Unit
        }.onFailure {
            close()
            Log.e(TAG, "USB connect failed", it)
        }
    }

    override suspend fun sendGestureEvent(event: GestureEvent): Result<ProtocolAck> = mutex.withLock {
        sendRawMessageLocked(event.toProtocolMessage())
    }

    override suspend fun sendRawMessage(raw: String): Result<ProtocolAck> = mutex.withLock {
        sendRawMessageLocked(raw)
    }

    private suspend fun sendRawMessageLocked(raw: String): Result<ProtocolAck> {
        return runCatching {
            val connection = activeConnection
                ?: throw UsbTransportException(context.getString(R.string.error_usb_disconnected))

            val ack = connection.exchange(raw)
            if (!ack.ok) {
                throw UsbTransportException(
                    context.getString(
                        R.string.error_usb_send_failed_format,
                        ack.message.ifBlank {
                            context.getString(R.string.error_usb_command_rejected)
                        },
                    ),
                )
            }
            ack
        }.onFailure { throwable ->
            if (throwable is IOException) {
                close()
            }
            Log.e(TAG, "USB gesture send failed", throwable)
        }
    }

    override fun disconnect() {
        close()
    }

    private fun findTouchBridgeAccessory(): UsbAccessory? {
        return usbManager.accessoryList
            ?.firstOrNull { accessory ->
                accessory.manufacturer == ACCESSORY_MANUFACTURER &&
                    accessory.model == ACCESSORY_MODEL
            }
    }

    private suspend fun openBestUsbConnection(): UsbConnection {
        if (!isCableConnected()) {
            throw UsbTransportException(context.getString(R.string.error_usb_cable_or_agent_required))
        }

        findTouchBridgeAccessory()?.let { accessory ->
            return openPermittedAccessoryConnection(accessory)
        }

        waitForTouchBridgeAccessoryOrNull()?.let { accessory ->
            return openPermittedAccessoryConnection(accessory)
        }

        discoverTcpEndpoint()?.let { endpoint ->
            return openTcpConnection(endpoint)
        }

        waitForTouchBridgeAccessoryOrNull()?.let { accessory ->
            return openPermittedAccessoryConnection(accessory)
        }

        throw UsbTransportException(context.getString(R.string.error_usb_cable_or_agent_required))
    }

    private suspend fun waitForTouchBridgeAccessoryOrNull(): UsbAccessory? {
        findTouchBridgeAccessory()?.let { return it }

        return try {
            withTimeout(USB_ACCESSORY_DISCOVERY_TIMEOUT_MS) {
                while (true) {
                    findTouchBridgeAccessory()?.let { return@withTimeout it }
                    delay(250)
                }

                @Suppress("UNREACHABLE_CODE")
                null
            }
        } catch (err: TimeoutCancellationException) {
            null
        }
    }

    private fun isCableConnected(): Boolean {
        val stickyUsbState = context.registerReceiver(
            null,
            IntentFilter(ACTION_USB_STATE),
        )

        return stickyUsbState?.getBooleanExtra(EXTRA_USB_CONNECTED, false) == true
    }

    private suspend fun requestAccessoryPermission(accessory: UsbAccessory): Boolean {
        return withContext(Dispatchers.Main) {
            val result = CompletableDeferred<Boolean>()
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    PendingIntent.FLAG_MUTABLE
                } else {
                    0
                }
            val permissionIntent = PendingIntent.getBroadcast(
                context,
                0,
                Intent(USB_PERMISSION_ACTION).setPackage(context.packageName),
                flags,
            )
            val receiver = object : BroadcastReceiver() {
                override fun onReceive(receiverContext: Context, intent: Intent) {
                    if (intent.action == USB_PERMISSION_ACTION && !result.isCompleted) {
                        result.complete(
                            intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false),
                        )
                    }
                }
            }

            ContextCompat.registerReceiver(
                context,
                receiver,
                IntentFilter(USB_PERMISSION_ACTION),
                ContextCompat.RECEIVER_NOT_EXPORTED,
            )

            try {
                usbManager.requestPermission(accessory, permissionIntent)
                withTimeout(USB_PERMISSION_TIMEOUT_MS) {
                    result.await()
                }
            } catch (err: TimeoutCancellationException) {
                throw UsbTransportException(context.getString(R.string.error_usb_permission_timeout))
            } finally {
                runCatching { context.unregisterReceiver(receiver) }
            }
        }
    }

    private suspend fun openPermittedAccessoryConnection(accessory: UsbAccessory): UsbConnection {
        if (!usbManager.hasPermission(accessory)) {
            val granted = requestAccessoryPermission(accessory)
            if (!granted) {
                throw UsbTransportException(context.getString(R.string.error_usb_permission_denied))
            }
        }

        return openAccessoryConnection(accessory)
    }

    private suspend fun openAccessoryConnection(accessory: UsbAccessory): UsbConnection {
        return withContext(Dispatchers.IO) {
            val descriptor = usbManager.openAccessory(accessory)
                ?: throw UsbTransportException(
                    context.getString(R.string.error_usb_open_failed),
                )
            UsbAccessoryConnection(
                descriptor = descriptor,
                noResponseMessage = context.getString(R.string.error_usb_no_response),
            )
        }
    }

    private suspend fun discoverTcpEndpoint(): UsbTcpEndpoint? {
        return withContext(Dispatchers.IO) {
            val now = System.currentTimeMillis()
            val beaconDeadline = now + USB_TCP_BEACON_DISCOVERY_TIMEOUT_MS
            val finalDeadline = now + USB_TCP_DISCOVERY_TIMEOUT_MS
            val usbNetworks = preferredUsbNetworks()
            val networks: List<Network?> = if (usbNetworks.isEmpty()) {
                listOf(null)
            } else {
                usbNetworks
            }

            for (network in networks) {
                listenForTcpBeacon(network, beaconDeadline)?.let { endpoint ->
                    return@withContext endpoint
                }
            }

            probeTcpEndpoints(tcpEndpointCandidates(usbNetworks))?.let { endpoint ->
                return@withContext endpoint
            }

            for (network in networks) {
                listenForTcpBeacon(network, finalDeadline)?.let { endpoint ->
                    return@withContext endpoint
                }
            }

            null
        }
    }

    @Suppress("DEPRECATION")
    private fun preferredUsbNetworks(): List<Network> {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return emptyList()
        }

        return connectivityManager.allNetworks.filter { network ->
            connectivityManager.getNetworkCapabilities(network)
                ?.hasTransport(NetworkCapabilities.TRANSPORT_USB) == true
        }
    }

    private fun listenForTcpBeacon(network: Network?, deadline: Long): UsbTcpEndpoint? {
        val buffer = ByteArray(1024)

        runCatching {
            DatagramSocket(null).use { socket ->
                if (network != null && Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    network.bindSocket(socket)
                }

                socket.reuseAddress = true
                socket.broadcast = true
                socket.soTimeout = 500
                socket.bind(InetSocketAddress(USB_TCP_BEACON_PORT))

                while (System.currentTimeMillis() < deadline) {
                    val packet = DatagramPacket(buffer, buffer.size)

                    try {
                        socket.receive(packet)
                        val raw = String(
                            packet.data,
                            packet.offset,
                            packet.length,
                            Charsets.UTF_8,
                        )
                        val json = JSONObject(raw)

                        if (json.optString("type") == USB_TCP_BEACON_TYPE) {
                            val port = json.optInt("port", 0)
                            val host = packet.address.hostAddress

                            if (!host.isNullOrBlank() && port > 0) {
                                return UsbTcpEndpoint(host, port, network)
                            }
                        }
                    } catch (_: SocketTimeoutException) {
                        // Keep listening until the discovery window expires.
                    } catch (err: Exception) {
                        Log.d(TAG, "Ignoring USB cable network beacon", err)
                    }
                }
            }
        }.onFailure { err ->
            Log.d(TAG, "USB cable network discovery failed", err)
        }

        return null
    }

    private suspend fun probeTcpEndpoints(candidates: List<UsbTcpEndpoint>): UsbTcpEndpoint? {
        if (candidates.isEmpty()) {
            return null
        }

        return coroutineScope {
            for (chunk in candidates.chunked(USB_TCP_SCAN_BATCH_SIZE)) {
                val results = chunk.map { endpoint ->
                    async(Dispatchers.IO) {
                        if (canOpenTcpEndpoint(endpoint)) endpoint else null
                    }
                }.awaitAll()

                results.firstOrNull { it != null }?.let { endpoint ->
                    return@coroutineScope endpoint
                }
            }

            null
        }
    }

    private fun canOpenTcpEndpoint(endpoint: UsbTcpEndpoint): Boolean {
        val socket = endpoint.network?.socketFactory?.createSocket() ?: Socket()
        return try {
            socket.connect(
                InetSocketAddress(endpoint.host, endpoint.port),
                USB_TCP_PROBE_TIMEOUT_MS,
            )
            true
        } catch (_: IOException) {
            false
        } finally {
            runCatching { socket.close() }
        }
    }

    private fun tcpEndpointCandidates(usbNetworks: List<Network>): List<UsbTcpEndpoint> {
        val candidates = linkedSetOf<UsbTcpEndpoint>()

        usbNetworks.forEach { network ->
            val linkProperties = connectivityManager.getLinkProperties(network)
            linkProperties?.linkAddresses
                ?.mapNotNull { linkAddress -> linkAddress.address as? Inet4Address }
                ?.forEach { address ->
                    addSubnetCandidates(candidates, address, network)
                }

            linkProperties?.routes
                ?.mapNotNull { route -> route.gateway as? Inet4Address }
                ?.forEach { gateway ->
                    candidates.add(UsbTcpEndpoint(gateway.hostAddress ?: return@forEach, USB_TCP_PORT, network))
                }
        }

        usbNetworkInterfaceAddresses().forEach { address ->
            addSubnetCandidates(candidates, address, null)
        }

        addClassCSubnetCandidates(candidates, "192.168.42")
        addClassCSubnetCandidates(candidates, "192.168.137")

        commonUsbTetherHosts().forEach { host ->
            candidates.add(UsbTcpEndpoint(host, USB_TCP_PORT, null))
        }

        return candidates.toList()
    }

    private fun addSubnetCandidates(
        candidates: MutableSet<UsbTcpEndpoint>,
        localAddress: Inet4Address,
        network: Network?,
    ) {
        val address = ipv4ToInt(localAddress)
        val subnet = address and 0xFFFFFF00.toInt()
        val local = address and 0xFF

        for (host in 1..254) {
            if (host == local) {
                continue
            }

            val candidateAddress = intToIpv4(subnet or host)
            candidates.add(UsbTcpEndpoint(candidateAddress.hostAddress ?: continue, USB_TCP_PORT, network))
        }
    }

    private fun addClassCSubnetCandidates(
        candidates: MutableSet<UsbTcpEndpoint>,
        prefix: String,
    ) {
        for (host in 1..254) {
            candidates.add(UsbTcpEndpoint("$prefix.$host", USB_TCP_PORT, null))
        }
    }

    private fun usbNetworkInterfaceAddresses(): List<Inet4Address> {
        return runCatching {
            NetworkInterface.getNetworkInterfaces().toList()
                .filter { networkInterface ->
                    val name = networkInterface.name.lowercase()
                    networkInterface.isUp &&
                        !networkInterface.isLoopback &&
                        (name.contains("usb") ||
                            name.contains("rndis") ||
                            name.contains("ncm") ||
                            name.contains("ecm") ||
                            name.startsWith("eth"))
                }
                .flatMap { networkInterface ->
                    networkInterface.inetAddresses.toList()
                        .filterIsInstance<Inet4Address>()
                        .filter { address -> !address.isLoopbackAddress }
                }
        }.getOrElse { err ->
            Log.d(TAG, "USB network interface scan failed", err)
            emptyList()
        }
    }

    private fun commonUsbTetherHosts(): List<String> {
        return listOf(
            "192.168.42.1",
            "192.168.42.2",
            "192.168.42.65",
            "192.168.42.129",
            "192.168.137.1",
            "172.20.10.1",
        )
    }

    private suspend fun openTcpConnection(endpoint: UsbTcpEndpoint): UsbConnection {
        return withContext(Dispatchers.IO) {
            val socket = endpoint.network?.socketFactory?.createSocket() ?: Socket()
            try {
                socket.connect(
                    InetSocketAddress(endpoint.host, endpoint.port),
                    USB_CONNECT_TIMEOUT_MS.toInt(),
                )
                socket.soTimeout = USB_CONNECT_TIMEOUT_MS.toInt()
                UsbTcpConnection(
                    socket = socket,
                    noResponseMessage = context.getString(R.string.error_usb_no_response),
                )
            } catch (err: IOException) {
                runCatching { socket.close() }
                throw err
            }
        }
    }

    private fun close() {
        activeConnection?.close()
        activeConnection = null
    }
}

private interface UsbConnection {
    val description: String

    suspend fun exchange(raw: String): ProtocolAck

    fun close()
}

private class UsbAccessoryConnection(
    private val descriptor: ParcelFileDescriptor,
    private val noResponseMessage: String,
) : UsbConnection {
    override val description: String = "AOA accessory"

    private val reader = BufferedReader(
        InputStreamReader(FileInputStream(descriptor.fileDescriptor), Charsets.UTF_8),
    )
    private val writer = BufferedWriter(
        OutputStreamWriter(FileOutputStream(descriptor.fileDescriptor), Charsets.UTF_8),
    )

    override suspend fun exchange(raw: String): ProtocolAck {
        return withTimeout(USB_CONNECT_TIMEOUT_MS) {
            withContext(Dispatchers.IO) {
                writer.write(raw)
                writer.newLine()
                writer.flush()

                val response = reader.readLine()
                    ?: throw UsbTransportException(noResponseMessage)
                parseProtocolAck(response)
            }
        }
    }

    override fun close() {
        runCatching { writer.close() }
        runCatching { reader.close() }
        runCatching { descriptor.close() }
    }
}

private class UsbTcpConnection(
    private val socket: Socket,
    private val noResponseMessage: String,
) : UsbConnection {
    override val description: String = "USB cable network"

    private val reader = BufferedReader(
        InputStreamReader(socket.getInputStream(), Charsets.UTF_8),
    )
    private val writer = BufferedWriter(
        OutputStreamWriter(socket.getOutputStream(), Charsets.UTF_8),
    )

    override suspend fun exchange(raw: String): ProtocolAck {
        return withTimeout(USB_CONNECT_TIMEOUT_MS) {
            withContext(Dispatchers.IO) {
                writer.write(raw)
                writer.newLine()
                writer.flush()

                val response = reader.readLine()
                    ?: throw UsbTransportException(noResponseMessage)
                parseProtocolAck(response)
            }
        }
    }

    override fun close() {
        runCatching { writer.close() }
        runCatching { reader.close() }
        runCatching { socket.close() }
    }
}

private data class UsbTcpEndpoint(
    val host: String,
    val port: Int,
    val network: Network?,
)

private fun ipv4ToInt(address: Inet4Address): Int {
    val bytes = address.address
    return ((bytes[0].toInt() and 0xFF) shl 24) or
        ((bytes[1].toInt() and 0xFF) shl 16) or
        ((bytes[2].toInt() and 0xFF) shl 8) or
        (bytes[3].toInt() and 0xFF)
}

private fun intToIpv4(value: Int): InetAddress {
    return InetAddress.getByAddress(
        byteArrayOf(
            ((value ushr 24) and 0xFF).toByte(),
            ((value ushr 16) and 0xFF).toByte(),
            ((value ushr 8) and 0xFF).toByte(),
            (value and 0xFF).toByte(),
        ),
    )
}

private class UsbTransportException(message: String) : IOException(message)
