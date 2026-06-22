mod config;
mod ebpf;
mod error;
mod sys;

use config::AgentConfig;
use ebpf::{EbpfConfig, EbpfManager};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: zerogate-agent --object <path> --iface <name> \
             [--program-name <name>] [--xdp-mode skb|driver|hardware]"
        );
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --object <path>         Path to the eBPF object file");
        eprintln!("  --iface <name>          Network interface name");
        eprintln!("  --program-name <name>   XDP program name (default: zerogate_xdp)");
        eprintln!(
            "  --xdp-mode <mode>       XDP attach mode: skb, driver, hardware (default: skb)"
        );
        std::process::exit(1);
    }

    let agent_config = match AgentConfig::from_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "ZeroGate agent starting (object={}, iface={}, program={}, mode={})",
        agent_config.ebpf_object_path,
        agent_config.interface_name,
        agent_config.program_name,
        agent_config.xdp_mode.as_str(),
    );

    let ebpf_config = EbpfConfig {
        object_path: agent_config.ebpf_object_path,
        interface_name: agent_config.interface_name,
        program_name: agent_config.program_name,
        attach_mode: agent_config.xdp_mode,
    };

    let mut mgr = EbpfManager::new(ebpf_config);

    println!("Validating configuration...");
    if let Err(e) = mgr.validate_config() {
        eprintln!("Config validation failed: {e}");
        std::process::exit(1);
    }

    println!("Loading eBPF program...");
    if let Err(e) = mgr.load() {
        eprintln!("eBPF load failed: {e}");
        eprintln!("Note: eBPF loading requires a Linux environment with a valid eBPF object.");
        std::process::exit(1);
    }

    println!("Attaching XDP program...");
    if let Err(e) = mgr.attach_xdp() {
        eprintln!("XDP attach failed: {e}");
        std::process::exit(1);
    }

    println!("XDP program attached. Waiting for shutdown signal...");

    // Block until interrupted. When Aya integration is complete, a proper
    // signal handler will call detach_xdp() before exiting. For now, the
    // loader will exit at load() above since the loader is not yet integrated.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
