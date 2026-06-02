package com.sharon77770.touchbridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceStoreTest {
    @Test
    fun scannedBleMergePreservesSavedNameAndTrust() {
        val saved = bleDeviceFromScan(
            id = bleDeviceId("AA:BB:CC:DD:EE:FF"),
            name = "Renamed PC",
            trusted = true,
            lastConnectedAt = 1234L,
        ).copy(
            paired = true,
            available = false,
            autoConnect = true,
        )
        val scanned = bleDeviceFromScan(
            id = saved.id,
            name = "TouchBridge BLE Agent",
        )

        val merged = mergeScannedBleDevices(listOf(saved), listOf(scanned))

        assertEquals(1, merged.size)
        assertEquals("Renamed PC", merged.single().name)
        assertTrue(merged.single().paired)
        assertTrue(merged.single().trusted)
        assertTrue(merged.single().available)
        assertTrue(merged.single().autoConnect)
        assertEquals(1234L, merged.single().lastConnectedAt)
    }
}
