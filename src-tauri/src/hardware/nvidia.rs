use serde::Serialize;

use crate::process::{self, CommandSpec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub total_memory_mib: u64,
    pub used_memory_mib: u64,
    pub free_memory_mib: u64,
    pub driver_version: Option<String>,
}

/// Fields requested from `nvidia-smi`, in the order the parser expects them.
pub const QUERY_FIELDS: &str = "index,name,memory.total,memory.used,memory.free,driver_version";

/// Parses `nvidia-smi --query-gpu=… --format=csv,noheader,nounits` output.
///
/// Rows that do not parse are skipped rather than failing the whole snapshot: a partially
/// readable GPU list is more useful than none.
pub fn parse_gpu_query(output: &str) -> Vec<GpuInfo> {
    output.lines().filter_map(parse_row).collect()
}

fn parse_row(line: &str) -> Option<GpuInfo> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() < 5 {
        return None;
    }

    let name = (*fields.get(1)?).to_string();
    if name.is_empty() {
        return None;
    }

    Some(GpuInfo {
        index: fields.first()?.parse().ok()?,
        name,
        total_memory_mib: fields.get(2)?.parse().ok()?,
        used_memory_mib: fields.get(3)?.parse().ok()?,
        free_memory_mib: fields.get(4)?.parse().ok()?,
        driver_version: fields
            .get(5)
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty()),
    })
}

/// Queries NVIDIA GPUs. Returns an empty list when `nvidia-smi` is absent or fails, because a
/// missing NVIDIA driver is a normal state, not an error.
pub async fn query_gpus() -> Vec<GpuInfo> {
    let spec = CommandSpec::new("nvidia-smi").args([
        format!("--query-gpu={QUERY_FIELDS}"),
        "--format=csv,noheader,nounits".to_string(),
    ]);

    match process::capture(&spec).await {
        Ok(output) if output.succeeded() => parse_gpu_query(&output.stdout),
        Ok(output) => {
            tracing::debug!(diagnostics = %output.diagnostics(), "nvidia-smi reported a failure");
            Vec::new()
        }
        Err(error) => {
            tracing::debug!(%error, "nvidia-smi is not available");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_GPUS: &str = "\
0, NVIDIA GeForce RTX 5090, 32607, 1024, 31583, 580.88
1, NVIDIA GeForce RTX 4090, 24564, 512, 24052, 580.88
";

    #[test]
    fn parses_every_reported_gpu() {
        let gpus = parse_gpu_query(TWO_GPUS);

        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].index, 0);
        assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 5090");
        assert_eq!(gpus[0].total_memory_mib, 32607);
        assert_eq!(gpus[0].free_memory_mib, 31583);
        assert_eq!(gpus[0].driver_version.as_deref(), Some("580.88"));
        assert_eq!(gpus[1].index, 1);
    }

    #[test]
    fn skips_rows_that_do_not_parse() {
        let output = "0, NVIDIA GeForce RTX 5090, 32607, 1024, 31583, 580.88\nnot a row\n";
        assert_eq!(parse_gpu_query(output).len(), 1);
    }

    #[test]
    fn tolerates_a_missing_driver_column() {
        let gpus = parse_gpu_query("0, NVIDIA GeForce RTX 5090, 32607, 1024, 31583\n");

        assert_eq!(gpus.len(), 1);
        assert!(gpus[0].driver_version.is_none());
    }

    #[test]
    fn empty_output_yields_no_gpus() {
        assert!(parse_gpu_query("").is_empty());
        assert!(parse_gpu_query("\n\n").is_empty());
    }
}
