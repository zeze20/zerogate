use crate::error::ZeroGateError;

/// Configuration for loading and attaching an eBPF/XDP program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfConfig {
    pub object_path: String,
    pub interface_name: String,
    pub program_name: String,
    pub attach_mode: XdpAttachMode,
}

/// XDP attach mode controlling how the program is hooked into the NIC path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpAttachMode {
    /// Generic/SKB mode — works on all interfaces, lowest performance.
    Skb,
    /// Native/driver mode — requires driver support, better performance.
    Driver,
    /// Hardware/offload mode — requires NIC firmware support.
    Hardware,
}

impl XdpAttachMode {
    pub fn from_str(s: &str) -> Result<Self, ZeroGateError> {
        match s {
            "skb" | "generic" => Ok(XdpAttachMode::Skb),
            "driver" | "native" => Ok(XdpAttachMode::Driver),
            "hardware" | "offload" => Ok(XdpAttachMode::Hardware),
            other => Err(ZeroGateError::InvalidConfig(format!(
                "unknown XDP attach mode: '{other}' (expected: skb, driver, hardware)"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            XdpAttachMode::Skb => "skb",
            XdpAttachMode::Driver => "driver",
            XdpAttachMode::Hardware => "hardware",
        }
    }
}

/// Lifecycle state of the eBPF manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfState {
    Created,
    Loaded,
    Attached,
    Detached,
}

/// Manages the userspace eBPF/XDP lifecycle: load, attach, detach.
///
/// State transitions: Created → Loaded → Attached → Detached.
/// Invalid transitions return structured errors.
///
/// Map access methods (policy, session, xsk) are placeholders for MR7.
#[allow(dead_code)]
pub struct EbpfManager {
    config: EbpfConfig,
    state: EbpfState,
}

#[allow(dead_code)]
impl EbpfManager {
    pub fn new(config: EbpfConfig) -> Self {
        Self {
            config,
            state: EbpfState::Created,
        }
    }

    pub fn config(&self) -> &EbpfConfig {
        &self.config
    }

    pub fn state(&self) -> EbpfState {
        self.state
    }

    /// Validate the eBPF configuration before attempting load.
    pub fn validate_config(&self) -> Result<(), ZeroGateError> {
        if self.config.object_path.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "eBPF object path must not be empty".to_string(),
            ));
        }
        if self.config.interface_name.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "interface name must not be empty".to_string(),
            ));
        }
        if self.config.program_name.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "program name must not be empty".to_string(),
            ));
        }
        if !std::path::Path::new(&self.config.object_path).exists() {
            return Err(ZeroGateError::BpfLoadFailed(format!(
                "eBPF object file not found: {}",
                self.config.object_path
            )));
        }
        Ok(())
    }

    /// Load the eBPF object from the configured path.
    ///
    /// Requires state == Created. Transitions to Loaded on success.
    /// Currently returns a structured error because real Aya-based loading
    /// requires a Linux environment with a valid eBPF ELF object.
    pub fn load(&mut self) -> Result<(), ZeroGateError> {
        if self.state != EbpfState::Created {
            return Err(ZeroGateError::InvalidEbpfState(format!(
                "load() requires state Created, current state: {:?}",
                self.state
            )));
        }

        self.validate_config()?;

        // Real Aya-based loading is deferred until the eBPF object is built
        // for bpfel-unknown-none on a Linux host. This scaffold validates
        // config and state transitions without faking a successful load.
        #[cfg(not(target_os = "linux"))]
        {
            Err(ZeroGateError::UnsupportedPlatform(
                "eBPF load requires Linux".to_string(),
            ))
        }

        #[cfg(target_os = "linux")]
        {
            // TODO: Integrate Aya loader when eBPF object is available.
            //   let mut bpf = aya::Bpf::load_file(&self.config.object_path)
            //       .map_err(|e| ZeroGateError::BpfLoadFailed(e.to_string()))?;
            Err(ZeroGateError::NotImplemented(
                "Aya eBPF loader integration is planned for a follow-up MR".to_string(),
            ))
        }
    }

    /// Attach the loaded XDP program to the configured interface.
    ///
    /// Requires state == Loaded. Transitions to Attached on success.
    pub fn attach_xdp(&mut self) -> Result<(), ZeroGateError> {
        if self.state != EbpfState::Loaded {
            return Err(ZeroGateError::InvalidEbpfState(format!(
                "attach_xdp() requires state Loaded, current state: {:?}",
                self.state
            )));
        }

        // Resolve interface index to validate the interface exists.
        let _ifindex = crate::sys::iface_index(&self.config.interface_name)?;

        // TODO: Aya XDP attach:
        //   let program: &mut Xdp = bpf.program_mut(&self.config.program_name)
        //       .ok_or_else(|| ZeroGateError::XdpAttachFailed(...))?
        //       .try_into()
        //       .map_err(|e| ZeroGateError::XdpAttachFailed(...))?;
        //   program.attach(&self.config.interface_name, xdp_flags)
        //       .map_err(|e| ZeroGateError::XdpAttachFailed(...))?;

        self.state = EbpfState::Attached;
        Ok(())
    }

    /// Detach the XDP program from the interface.
    ///
    /// Requires state == Attached. Transitions to Detached on success.
    pub fn detach_xdp(&mut self) -> Result<(), ZeroGateError> {
        if self.state != EbpfState::Attached {
            return Err(ZeroGateError::InvalidEbpfState(format!(
                "detach_xdp() requires state Attached, current state: {:?}",
                self.state
            )));
        }

        // TODO: Aya XDP detach — the link is dropped automatically when
        // the Aya Bpf object is dropped, but explicit detach is preferred.

        self.state = EbpfState::Detached;
        Ok(())
    }

    /// Open the policy BPF map for reading/writing.
    ///
    /// Real kernel POLICY map binding requires the Aya loader to expose
    /// map handles. Until the loader is fully integrated, this returns
    /// `NotImplemented`.
    pub fn open_policy_map(&mut self) -> Result<(), ZeroGateError> {
        Err(ZeroGateError::NotImplemented(
            "real kernel POLICY map binding is planned for a future MR".to_string(),
        ))
    }

    /// Open the session BPF map for reading/writing.
    ///
    /// Real kernel SESSIONS map binding requires the Aya loader to expose
    /// map handles. Until the loader is fully integrated, this returns
    /// `NotImplemented`.
    pub fn open_session_map(&mut self) -> Result<(), ZeroGateError> {
        Err(ZeroGateError::NotImplemented(
            "real kernel SESSIONS map binding is planned for a future MR".to_string(),
        ))
    }

    /// Open the XSK BPF map for AF_XDP socket registration.
    ///
    /// Real kernel XSK_MAP binding requires both the Aya loader and
    /// AF_XDP socket file descriptors. Until those are available, this
    /// returns `NotImplemented`.
    pub fn open_xsk_map(&mut self) -> Result<(), ZeroGateError> {
        Err(ZeroGateError::NotImplemented(
            "real kernel XSK_MAP binding is planned for a future MR".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EbpfConfig {
        EbpfConfig {
            object_path: "/tmp/test.o".to_string(),
            interface_name: "eth0".to_string(),
            program_name: "zerogate_xdp".to_string(),
            attach_mode: XdpAttachMode::Skb,
        }
    }

    #[test]
    fn manager_starts_in_created_state() {
        let mgr = EbpfManager::new(test_config());
        assert_eq!(mgr.state(), EbpfState::Created);
    }

    #[test]
    fn config_accessor_returns_config() {
        let cfg = test_config();
        let mgr = EbpfManager::new(cfg.clone());
        assert_eq!(mgr.config(), &cfg);
    }

    #[test]
    fn attach_before_load_returns_error() {
        let mut mgr = EbpfManager::new(test_config());
        match mgr.attach_xdp() {
            Err(ZeroGateError::InvalidEbpfState(_)) => {}
            other => panic!("expected InvalidEbpfState, got: {other:?}"),
        }
    }

    #[test]
    fn detach_before_attach_returns_error() {
        let mut mgr = EbpfManager::new(test_config());
        match mgr.detach_xdp() {
            Err(ZeroGateError::InvalidEbpfState(_)) => {}
            other => panic!("expected InvalidEbpfState, got: {other:?}"),
        }
    }

    #[test]
    fn policy_map_returns_not_implemented() {
        let mut mgr = EbpfManager::new(test_config());
        match mgr.open_policy_map() {
            Err(ZeroGateError::NotImplemented(msg)) => {
                assert!(msg.contains("POLICY"));
            }
            other => panic!("expected NotImplemented, got: {other:?}"),
        }
    }

    #[test]
    fn session_map_returns_not_implemented() {
        let mut mgr = EbpfManager::new(test_config());
        match mgr.open_session_map() {
            Err(ZeroGateError::NotImplemented(msg)) => {
                assert!(msg.contains("SESSIONS"));
            }
            other => panic!("expected NotImplemented, got: {other:?}"),
        }
    }

    #[test]
    fn xsk_map_returns_not_implemented() {
        let mut mgr = EbpfManager::new(test_config());
        match mgr.open_xsk_map() {
            Err(ZeroGateError::NotImplemented(msg)) => {
                assert!(msg.contains("XSK_MAP"));
            }
            other => panic!("expected NotImplemented, got: {other:?}"),
        }
    }

    #[test]
    fn xdp_attach_mode_as_str() {
        assert_eq!(XdpAttachMode::Skb.as_str(), "skb");
        assert_eq!(XdpAttachMode::Driver.as_str(), "driver");
        assert_eq!(XdpAttachMode::Hardware.as_str(), "hardware");
    }

    #[test]
    fn xdp_attach_mode_roundtrip() {
        for mode in &[
            XdpAttachMode::Skb,
            XdpAttachMode::Driver,
            XdpAttachMode::Hardware,
        ] {
            assert_eq!(XdpAttachMode::from_str(mode.as_str()).unwrap(), *mode);
        }
    }

    #[test]
    fn xdp_attach_mode_aliases() {
        assert_eq!(
            XdpAttachMode::from_str("generic").unwrap(),
            XdpAttachMode::Skb
        );
        assert_eq!(
            XdpAttachMode::from_str("native").unwrap(),
            XdpAttachMode::Driver
        );
        assert_eq!(
            XdpAttachMode::from_str("offload").unwrap(),
            XdpAttachMode::Hardware
        );
    }

    #[test]
    fn error_display_strings_are_non_empty() {
        let errors = [
            ZeroGateError::BpfLoadFailed("test".to_string()),
            ZeroGateError::XdpAttachFailed("test".to_string()),
            ZeroGateError::XdpDetachFailed("test".to_string()),
            ZeroGateError::MapOpenFailed("test".to_string()),
            ZeroGateError::MapUpdateFailed("test".to_string()),
            ZeroGateError::MapDeleteFailed("test".to_string()),
            ZeroGateError::InterfaceResolveFailed("test".to_string()),
            ZeroGateError::InvalidEbpfState("test".to_string()),
            ZeroGateError::UnsupportedPlatform("test".to_string()),
            ZeroGateError::NotImplemented("test".to_string()),
            ZeroGateError::InvalidConfig("test".to_string()),
            ZeroGateError::InvalidPolicy("test".to_string()),
            ZeroGateError::InvalidSession("test".to_string()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Display for {err:?} should be non-empty");
        }
    }
}
