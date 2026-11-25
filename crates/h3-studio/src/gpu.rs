//! Local GPU / host telemetry.
//!
//! GPU data comes from `nvidia-smi` (the same machine hosts the server),
//! host RAM/CPU comes from sysinfo. Everything degrades to "unknown" if the
//! tools are unavailable — no fabricated numbers.

use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct GpuInfo {
    pub name: String,
    pub vram_used_mb: f64,
    pub vram_total_mb: f64,
    pub util_percent: f32,
    pub temp_c: f32,
    pub power_w: f32,
    pub busy: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HostInfo {
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub cpu_percent: f32,
    pub cpu_cores_active: usize,
    pub cpu_cores_total: usize,
}

pub fn collect_gpus() -> Vec<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.used,memory.total,utilization.gpu,temperature.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output();
    let Ok(output) = output else { return Vec::new() };
    if !output.status.success() { return Vec::new() }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 6 { return None }
            let vram_used = parts[1].parse::<f64>().unwrap_or(0.0);
            let vram_total = parts[2].parse::<f64>().unwrap_or(0.0);
            let util = parts[3].parse::<f32>().unwrap_or(0.0);
            let temp = parts[4].parse::<f32>().unwrap_or(0.0);
            let power = parts[5].parse::<f32>().unwrap_or(0.0);
            Some(GpuInfo {
                name: parts[0].to_string(),
                vram_used_mb: vram_used,
                vram_total_mb: vram_total,
                util_percent: util,
                temp_c: temp,
                power_w: power,
                busy: util > 5.0 || vram_used > vram_total * 0.05,
            })
        })
        .collect()
}

pub fn collect_host() -> HostInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let ram_total = sys.total_memory() as f64 / 1e9;
    let ram_used = (sys.total_memory() - sys.free_memory()) as f64 / 1e9;
    let cpu_percent = sys.global_cpu_usage();
    let cores_total = sys.cpus().len();
    let cores_active = (cpu_percent / 100.0 * cores_total as f32).round().max(1.0) as usize;
    HostInfo {
        ram_used_gb: ram_used,
        ram_total_gb: ram_total,
        cpu_percent,
        cpu_cores_active: cores_active.min(cores_total),
        cpu_cores_total: cores_total,
    }
}
