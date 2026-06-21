// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Low-level system interfaces.
//!
//! This is one of two files in `zerogate-agent` allowed to contain `unsafe`.
//! All unsafe blocks have full safety justification comments.
//!
//! AF_XDP functions are Linux-only and gated with `#[cfg(target_os = "linux")]`.

/// Resolves a network interface name to its OS index.
///
/// Returns the interface index on success.
#[cfg(target_os = "linux")]
pub fn iface_index(name: &str) -> Result<u32, crate::error::ZeroGateError> {
    let c_name = std::ffi::CString::new(name).map_err(|_| {
        crate::error::ZeroGateError::System(format!("interface name '{name}' contains a null byte"))
    })?;

    // SAFETY:
    // - Provenance: c_name is a valid, null-terminated CString.
    // - Bounds: CString::as_ptr returns a pointer to the full string
    //   including the null terminator.
    // - Alignment: char pointer, no alignment requirement.
    // - Lifetime: c_name is alive for the duration of this call.
    // - Aliasing: if_nametoindex is read-only on the string.
    let idx = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    if idx == 0 {
        return Err(crate::error::ZeroGateError::System(format!(
            "interface '{name}' not found: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(idx)
}

/// Stub for non-Linux platforms.
#[cfg(not(target_os = "linux"))]
pub fn iface_index(name: &str) -> Result<u32, crate::error::ZeroGateError> {
    Err(crate::error::ZeroGateError::System(format!(
        "iface_index not supported on this platform (requested: '{name}')"
    )))
}
