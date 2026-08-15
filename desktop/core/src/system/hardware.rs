//! RAM/CPU profile + small-model size recommendation.

use sysinfo::System;

#[derive(Debug, Clone, Copy)]
pub struct HardwareProfile {
    pub total_ram_gb: f64,
    pub free_ram_gb: f64,
    pub cpu_cores: usize,
}

pub fn get_hardware_profile() -> HardwareProfile {
    let mut sys = System::new_all();
    sys.refresh_all();
    const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;
    HardwareProfile {
        total_ram_gb: sys.total_memory() as f64 / BYTES_PER_GB,
        free_ram_gb: sys.free_memory() as f64 / BYTES_PER_GB,
        cpu_cores: sys.cpus().len(),
    }
}

/// Name as passed to `ollama pull`.
#[derive(Debug, Clone, Copy)]
pub struct ModelRecommendation {
    pub name: &'static str,
    pub approx_size_gb: f64,
    pub description: &'static str,
}

// All small enough to be a reasonable default download; picked by available RAM so the
// recommendation still fits comfortably rather than swapping to disk.
const TIERS: [ModelRecommendation; 3] = [
    ModelRecommendation {
        name: "qwen2.5-coder:1.5b",
        approx_size_gb: 1.0,
        description: "tiny & fast — great for a first test on modest hardware",
    },
    ModelRecommendation {
        name: "qwen2.5-coder:3b",
        approx_size_gb: 1.9,
        description: "still small, noticeably better coding quality",
    },
    ModelRecommendation {
        name: "qwen2.5-coder:7b",
        approx_size_gb: 4.7,
        description: "the strongest of the small tiers, wants more RAM",
    },
];

pub struct ModelRecommendationResult {
    pub recommended: ModelRecommendation,
    pub alternatives: Vec<ModelRecommendation>,
}

pub fn recommend_model(hw: &HardwareProfile) -> ModelRecommendationResult {
    let idx = if hw.total_ram_gb >= 16.0 {
        2
    } else if hw.total_ram_gb >= 8.0 {
        1
    } else {
        0
    };
    ModelRecommendationResult {
        recommended: TIERS[idx],
        alternatives: TIERS.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, m)| *m).collect(),
    }
}
