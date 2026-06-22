use crate::error::ZeroGateError;

/// Resolve a network interface name to its OS index.
///
/// On Linux, this calls `libc::if_nametoindex` via FFI.
/// On non-Linux platforms, returns `UnsupportedPlatform`.
pub fn iface_index(name: &str) -> Result<u32, ZeroGateError> {
    if name.is_empty() {
        return Err(ZeroGateError::InterfaceResolveFailed(
            "interface name is empty".to_string(),
        ));
    }

    // Reject interior NUL bytes before passing to C.
    if name.contains('\0') {
        return Err(ZeroGateError::InterfaceResolveFailed(
            "interface name contains NUL byte".to_string(),
        ));
    }

    #[cfg(target_os = "linux")]
    {
        use std::ffi::CString;

        let c_name = CString::new(name).map_err(|e| {
            ZeroGateError::InterfaceResolveFailed(format!("invalid interface name: {e}"))
        })?;

        // SAFETY: `c_name` is a valid NUL-terminated C string.
        // `if_nametoindex` reads the string without modifying it.
        // The pointer is valid for the duration of the FFI call.
        // The function returns 0 on failure (no matching interface),
        // or a positive u32 interface index on success.
        let idx = unsafe { libc::if_nametoindex(c_name.as_ptr()) };

        if idx == 0 {
            return Err(ZeroGateError::InterfaceResolveFailed(format!(
                "interface '{name}' not found (if_nametoindex returned 0)"
            )));
        }

        Ok(idx)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(ZeroGateError::UnsupportedPlatform(
            "interface index resolution requires Linux".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        assert!(iface_index("").is_err());
    }

    #[test]
    fn rejects_nul_in_name() {
        assert!(iface_index("eth\x000").is_err());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_returns_unsupported() {
        match iface_index("eth0") {
            Err(ZeroGateError::UnsupportedPlatform(_)) => {}
            other => panic!("expected UnsupportedPlatform, got: {other:?}"),
        }
    }
}
