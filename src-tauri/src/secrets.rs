use crate::{db::Store, Failure, Result};

#[cfg(windows)]
fn crypt(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(Failure::new(
            2,
            "DPAPI 操作失败；请使用加密时的 Windows 用户",
        ));
    }
    let out = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as *mut std::ffi::c_void);
    };
    Ok(out)
}
#[cfg(not(windows))]
fn crypt(_: &[u8], _: bool) -> Result<Vec<u8>> {
    Err(Failure::new(2, "Token 存储只支持 Windows DPAPI"))
}
pub fn save(s: &Store, token: &str) -> Result<()> {
    let p = s.dir.join("token.dpapi");
    if token.is_empty() {
        if p.exists() {
            std::fs::remove_file(p)?;
        }
        return Ok(());
    }
    if token.len() > 4096
        || !token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(Failure::new(1, "Token 格式无效"));
    }
    let encrypted = crypt(token.as_bytes(), true)?;
    // 不产生明文临时文件。
    std::fs::write(p, encrypted)?;
    Ok(())
}
pub fn load(s: &Store) -> Result<Option<String>> {
    let p = s.dir.join("token.dpapi");
    if !p.exists() {
        return Ok(None);
    }
    let bytes = crypt(&std::fs::read(p)?, false)?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Failure::new(5, "Token 解密格式无效"))
}
pub fn redact(text: &str, token: Option<&str>) -> String {
    use base64::Engine;
    let mut out = text.to_string();
    if let Some(t) = token {
        out = out.replace(t, "[REDACTED]").replace(
            &base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{t}")),
            "[REDACTED]",
        );
    }
    for prefix in ["ghp_", "github_pat_", "gho_", "ghs_", "ghu_", "ghr_"] {
        while let Some(start) = out.find(prefix) {
            let end = out[start..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .map_or(out.len(), |i| start + i);
            out.replace_range(start..end, "[REDACTED]");
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removes_tokens() {
        assert_eq!(redact("fatal ghp_secret123!", None), "fatal [REDACTED]!");
    }
    #[test]
    #[cfg(windows)]
    fn dpapi_roundtrip() {
        let bytes = crypt(b"secret", true).unwrap();
        assert_ne!(bytes, b"secret");
        assert_eq!(crypt(&bytes, false).unwrap(), b"secret");
    }
}
