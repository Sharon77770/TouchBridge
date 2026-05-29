use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, SetLastError, WIN32_ERROR,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;

use crate::tray;

pub struct SingleInstanceGuard {
    mutex: HANDLE,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.mutex);
        }
    }
}

pub fn claim_or_show_existing() -> windows::core::Result<Option<SingleInstanceGuard>> {
    unsafe {
        SetLastError(WIN32_ERROR(0));
    }

    let mutex = unsafe { CreateMutexW(None, true, w!("Local\\TouchBridgeAgent.SingleInstance"))? };

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(mutex);
        }
        let _ = tray::request_existing_instance_show();
        return Ok(None);
    }

    Ok(Some(SingleInstanceGuard { mutex }))
}
