use std::{mem::size_of, os::windows::ffi::OsStrExt};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
            RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        },
    },
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "CodexGaugeNative";

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let key = open_write_key()?;
    let result = if enabled {
        let executable = std::env::current_exe().map_err(|_| "autostart_path")?;
        let command = format!("\"{}\" --autostart", executable.display());
        let command_wide = wide(&command);
        let bytes = unsafe {
            std::slice::from_raw_parts(
                command_wide.as_ptr().cast::<u8>(),
                command_wide.len() * size_of::<u16>(),
            )
        };
        unsafe {
            RegSetValueExW(
                key,
                PCWSTR(wide(VALUE_NAME).as_ptr()),
                None,
                REG_SZ,
                Some(bytes),
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, PCWSTR(wide(VALUE_NAME).as_ptr())) }
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if result == ERROR_SUCCESS || (!enabled && result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err("autostart_registry".to_string())
    }
}

pub fn is_enabled() -> bool {
    let mut key = HKEY::default();
    let mut size = 0u32;
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide(RUN_KEY).as_ptr()),
            None,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if result != ERROR_SUCCESS {
        return false;
    }
    let result = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(wide(VALUE_NAME).as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    result == ERROR_SUCCESS
}

fn open_write_key() -> Result<HKEY, String> {
    let mut key = HKEY::default();
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide(RUN_KEY).as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };
    if result == ERROR_SUCCESS {
        Ok(key)
    } else {
        Err("autostart_registry".to_string())
    }
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}
