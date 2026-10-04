//! Encryption of saved passwords with the Windows account's own key (DPAPI): only the same user on the same
//! machine can decrypt them, and no key is ever stored by the browser.

/// Extra data mixed into the key so other programs' DPAPI blobs cannot be fed to us (and vice versa).
const ENTROPY: &[u8] = b"AxomaiBrowser.passwords.v1";

#[cfg(windows)]
mod imp {
    use super::ENTROPY;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
        v
    }

    pub fn protect(plain: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let (input, entropy) = (blob(plain), blob(ENTROPY));
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(&input, None, Some(&entropy), None, None, 0, &mut out).ok()?;
            Some(take(out))
        }
    }

    pub fn unprotect(cipher: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let (input, entropy) = (blob(cipher), blob(ENTROPY));
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(&input, None, Some(&entropy), None, None, 0, &mut out).ok()?;
            Some(take(out))
        }
    }
}

#[cfg(not(windows))]
mod imp {
    /// No OS key store here: refuse to store anything rather than keep passwords in the clear.
    pub fn protect(_plain: &[u8]) -> Option<Vec<u8>> {
        None
    }
    pub fn unprotect(_cipher: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

pub fn encrypt(text: &str) -> Option<Vec<u8>> {
    imp::protect(text.as_bytes())
}

pub fn decrypt(blob: &[u8]) -> Option<String> {
    String::from_utf8(imp::unprotect(blob)?).ok()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_not_plaintext() {
        let blob = encrypt("hunter2 \u{1F511}").unwrap();
        assert!(!blob.windows(7).any(|w| w == b"hunter2"), "ciphertext must not contain the password");
        assert_eq!(decrypt(&blob).as_deref(), Some("hunter2 \u{1F511}"));
    }

    #[test]
    fn tampered_or_foreign_data_is_rejected() {
        let mut blob = encrypt("secret").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0xFF;
        assert_eq!(decrypt(&blob), None);
        assert_eq!(decrypt(b"not a dpapi blob"), None);
    }
}
