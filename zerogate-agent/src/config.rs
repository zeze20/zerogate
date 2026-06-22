use crate::ebpf::XdpAttachMode;
use crate::error::ZeroGateError;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub ebpf_object_path: String,
    pub interface_name: String,
    pub program_name: String,
    pub xdp_mode: XdpAttachMode,
}

impl AgentConfig {
    pub fn validate(&self) -> Result<(), ZeroGateError> {
        if self.ebpf_object_path.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "eBPF object path must not be empty".to_string(),
            ));
        }
        if self.interface_name.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "interface name must not be empty".to_string(),
            ));
        }
        if self.program_name.is_empty() {
            return Err(ZeroGateError::InvalidConfig(
                "program name must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn from_args(args: &[String]) -> Result<Self, ZeroGateError> {
        let mut object_path = String::new();
        let mut iface = String::new();
        let mut program = "zerogate_xdp".to_string();
        let mut mode = XdpAttachMode::Skb;

        let mut i = 1; // skip argv[0]
        while i < args.len() {
            match args[i].as_str() {
                "--object" if i + 1 < args.len() => {
                    object_path = args[i + 1].clone();
                    i += 2;
                }
                "--iface" if i + 1 < args.len() => {
                    iface = args[i + 1].clone();
                    i += 2;
                }
                "--program-name" if i + 1 < args.len() => {
                    program = args[i + 1].clone();
                    i += 2;
                }
                "--xdp-mode" if i + 1 < args.len() => {
                    mode = XdpAttachMode::from_str(&args[i + 1])?;
                    i += 2;
                }
                other => {
                    return Err(ZeroGateError::InvalidConfig(format!(
                        "unknown argument: {other}"
                    )));
                }
            }
        }

        let config = AgentConfig {
            ebpf_object_path: object_path,
            interface_name: iface,
            program_name: program,
            xdp_mode: mode,
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_object_path() {
        let config = AgentConfig {
            ebpf_object_path: String::new(),
            interface_name: "eth0".to_string(),
            program_name: "zerogate_xdp".to_string(),
            xdp_mode: XdpAttachMode::Skb,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_empty_interface_name() {
        let config = AgentConfig {
            ebpf_object_path: "/path/to/obj".to_string(),
            interface_name: String::new(),
            program_name: "zerogate_xdp".to_string(),
            xdp_mode: XdpAttachMode::Skb,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_empty_program_name() {
        let config = AgentConfig {
            ebpf_object_path: "/path/to/obj".to_string(),
            interface_name: "eth0".to_string(),
            program_name: String::new(),
            xdp_mode: XdpAttachMode::Skb,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn accepts_valid_config() {
        let config = AgentConfig {
            ebpf_object_path: "/path/to/obj".to_string(),
            interface_name: "eth0".to_string(),
            program_name: "zerogate_xdp".to_string(),
            xdp_mode: XdpAttachMode::Skb,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn parse_xdp_mode_skb() {
        assert_eq!(XdpAttachMode::from_str("skb").unwrap(), XdpAttachMode::Skb);
    }

    #[test]
    fn parse_xdp_mode_driver() {
        assert_eq!(
            XdpAttachMode::from_str("driver").unwrap(),
            XdpAttachMode::Driver
        );
    }

    #[test]
    fn parse_xdp_mode_hardware() {
        assert_eq!(
            XdpAttachMode::from_str("hardware").unwrap(),
            XdpAttachMode::Hardware
        );
    }

    #[test]
    fn parse_xdp_mode_invalid() {
        assert!(XdpAttachMode::from_str("invalid").is_err());
    }
}
