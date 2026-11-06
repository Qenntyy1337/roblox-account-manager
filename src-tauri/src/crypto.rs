//! Шифрование секретных данных через Windows DPAPI (область CurrentUser).
//!
//! Cookie .ROBLOSECURITY шифруется функцией CryptProtectData с дополнительной
//! энтропией приложения — расшифровать данные может только тот же пользователь
//! Windows на той же машине. На не-Windows платформах (разработка/превью)
//! используется прозрачный fallback без шифрования.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

#[cfg(windows)]
const DPAPI_FLAGS: u32 = 0x1; // CRYPTPROTECT_UI_FORBIDDEN
#[cfg(windows)]
const ENTROPY: &[u8] = b"RobloxAccountManager::v1";

#[cfg(windows)]
#[repr(C)]
struct CryptBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[cfg(windows)]
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        p_data_in: *const CryptBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *const CryptBlob,
        pv_reserved: *mut std::ffi::c_void,
        p_prompt_struct: *mut std::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut CryptBlob,
    ) -> i32;

    fn CryptUnprotectData(
        p_data_in: *const CryptBlob,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *const CryptBlob,
        pv_reserved: *mut std::ffi::c_void,
        p_prompt_struct: *mut std::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut CryptBlob,
    ) -> i32;

    fn LocalFree(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = CryptBlob {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let entropy = CryptBlob {
            cb_data: ENTROPY.len() as u32,
            pb_data: ENTROPY.as_ptr() as *mut u8,
        };
        let mut out = CryptBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = CryptProtectData(
            &input,
            std::ptr::null(),
            &entropy,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            DPAPI_FLAGS,
            &mut out,
        );
        if ok == 0 {
            return Err("CryptProtectData: вызов завершился ошибкой".into());
        }
        let bytes =
            std::slice::from_raw_parts(out.pb_data, out.cb_data as usize).to_vec();
        LocalFree(out.pb_data as *mut std::ffi::c_void);
        Ok(bytes)
    }
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = CryptBlob {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let entropy = CryptBlob {
            cb_data: ENTROPY.len() as u32,
            pb_data: ENTROPY.as_ptr() as *mut u8,
        };
        let mut out = CryptBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            &entropy,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            DPAPI_FLAGS,
            &mut out,
        );
        if ok == 0 {
            return Err("CryptUnprotectData: вызов завершился ошибкой".into());
        }
        let bytes =
            std::slice::from_raw_parts(out.pb_data, out.cb_data as usize).to_vec();
        LocalFree(out.pb_data as *mut std::ffi::c_void);
        Ok(bytes)
    }
}

#[cfg(not(windows))]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    // Fallback для разработки вне Windows: данные не шифруются
    Ok(data.to_vec())
}

#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

/// Зашифровать cookie и вернуть base64-строку для хранения в config.json
pub fn encrypt_cookie(cookie: &str) -> String {
    match protect(cookie.as_bytes()) {
        Ok(b) => B64.encode(b),
        Err(_) => String::new(),
    }
}

/// Расшифровать cookie из хранилища
pub fn decrypt_cookie(enc: &str) -> Option<String> {
    if enc.is_empty() {
        return None;
    }
    let raw = B64.decode(enc).ok()?;
    let plain = unprotect(&raw).ok()?;
    String::from_utf8(plain).ok()
}
