// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! ZeroGate AF_XDP userspace agent entry point.
//!
//! No `unsafe` in this file. All unsafe is isolated in `umem.rs` and `sys.rs`.

#![allow(dead_code)]

mod config;
mod cpu;
mod error;
mod frame_pool;
mod policy;
mod queue;
mod rings;
mod stats;
mod sys;
mod umem;
mod xsk;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use log::info;

use config::{AgentConfig, XdpMode};
use frame_pool::FramePool;
use umem::{UmemConfig, UmemRegion};

/// ZeroGate AF_XDP userspace agent.
#[derive(Parser, Debug)]
#[command(name = "zerogate-agent", about = "ZeroGate AF_XDP userspace agent")]
struct Cli {
    /// Path to the compiled eBPF object file.
    #[arg(short = 'o', long)]
    ebpf_obj: String,

    /// Network interface name.
    #[arg(short, long)]
    iface: String,

    /// NIC queue IDs (comma-separated).
    #[arg(short, long, value_delimiter = ',', default_value = "0")]
    queues: Vec<u32>,

    /// CPU cores to pin queues to (comma-separated, same length as queues).
    #[arg(short, long, value_delimiter = ',', default_value = "0")]
    cpus: Vec<usize>,

    /// Session IDs to pre-admit (comma-separated hex or decimal).
    #[arg(short, long, value_delimiter = ',')]
    sessions: Vec<u64>,

    /// XDP attach mode.
    #[arg(long, default_value = "skb")]
    mode: String,

    /// Number of UMEM frames per queue.
    #[arg(long, default_value_t = 4096)]
    frame_count: u32,

    /// UMEM frame size in bytes.
    #[arg(long, default_value_t = 4096)]
    frame_size: u32,

    /// Force copy mode (disable zero-copy).
    #[arg(long, default_value_t = false)]
    force_copy: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let xdp_mode = match cli.mode.as_str() {
        "skb" => XdpMode::Skb,
        "drv" | "driver" => XdpMode::Driver,
        "hw" | "hardware" => XdpMode::Hardware,
        other => anyhow::bail!("unknown XDP mode: {other}"),
    };

    let config = AgentConfig {
        ebpf_obj: cli.ebpf_obj,
        iface: cli.iface,
        queue_ids: cli.queues,
        cpu_ids: cli.cpus,
        frame_count: cli.frame_count,
        frame_size: cli.frame_size,
        force_copy: cli.force_copy,
        xdp_mode,
        sessions: cli.sessions,
    };

    config.validate().map_err(|e| anyhow::anyhow!("{e}"))?;

    info!("ZeroGate agent starting");
    info!("  interface: {}", config.iface);
    info!("  queues: {:?}", config.queue_ids);
    info!("  CPUs: {:?}", config.cpu_ids);
    info!("  frame_count: {}", config.frame_count);
    info!("  frame_size: {}", config.frame_size);
    info!("  mode: {:?}", config.xdp_mode);
    info!("  sessions: {:?}", config.sessions);

    // --- Step 1: Load & attach eBPF program ---
    // TODO: On Linux, use aya to load the eBPF object and attach XDP.
    // This requires the aya crate which depends on Linux-specific APIs.
    info!("eBPF program: {} (load TODO on Linux)", config.ebpf_obj);

    // --- Step 2: Populate SESSIONS map ---
    // TODO: On Linux, insert session keys into the BPF HashMap.
    for &sid in &config.sessions {
        info!("admitting session_id={sid:#018x}");
    }

    // --- Step 3: Set up per-queue resources ---
    let shutdown = Arc::new(AtomicBool::new(false));

    // Install Ctrl+C handler.
    let shutdown_flag = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("shutdown signal received");
        shutdown_flag.store(true, Ordering::SeqCst);
    })
    .expect("failed to install Ctrl+C handler");

    let mut handles = Vec::new();

    for (i, &queue_id) in config.queue_ids.iter().enumerate() {
        let cpu_id = config.cpu_ids[i];
        let frame_count = config.frame_count;
        let frame_size = config.frame_size;
        let shutdown_clone = shutdown.clone();

        let handle = std::thread::Builder::new()
            .name(format!("zg-queue-{queue_id}"))
            .spawn(move || {
                let pool = FramePool::new(frame_count, frame_size);
                let umem = UmemRegion::allocate(UmemConfig {
                    frame_count,
                    frame_size,
                })
                .expect("UMEM allocation failed");

                let ctx = queue::QueueContext::new(queue_id, cpu_id, pool, umem);
                queue::run_queue_loop(ctx, shutdown_clone)
            })?;

        handles.push(handle);
    }

    // Wait for all queue threads to finish.
    let mut all_stats = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(ctx) => {
                info!("queue {} finished: {}", ctx.queue_id, ctx.stats);
                all_stats.push(ctx.stats);
            }
            Err(_) => {
                log::error!("queue thread panicked");
            }
        }
    }

    // Aggregate stats.
    let agg = stats::AggregateStats::from_queues(&all_stats);
    info!("shutdown complete. {agg}");

    Ok(())
}
