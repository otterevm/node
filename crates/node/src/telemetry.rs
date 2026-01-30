//! Unified telemetry module for exporting metrics from both consensus and execution layers.
//!
//! This module pushes Prometheus-format metrics directly to Victoria Metrics by polling:
//! - Commonware's runtime context (`context.encode()`)
//! - Reth's prometheus recorder (`handle.render()`)

use std::collections::HashMap;

use commonware_runtime::{Metrics as _, Spawner as _, tokio::Context};
use eyre::WrapErr as _;
use jiff::SignedDuration;
use reth_node_metrics::recorder::install_prometheus_recorder;
use reth_tracing::tracing;

/// Configuration for Prometheus metrics push export.
pub struct PrometheusMetricsConfig {
    /// The Prometheus export endpoint.
    pub endpoint: String,
    /// The interval at which to push metrics.
    pub interval: SignedDuration,
    /// Labels to add to all metrics if possible -- i.e VictoriaMetrics `extra_labels` query string.
    pub labels: HashMap<String, String>,
}

/// Spawns a task that periodically pushes both consensus and execution metrics to Victoria Metrics.
///
/// This concatenates Prometheus-format metrics from both sources and pushes them directly
/// to Victoria Metrics' Prometheus import endpoint.
///
/// The task runs for the lifetime of the consensus runtime.
pub fn install_prometheus_metrics(
    context: Context,
    config: PrometheusMetricsConfig,
) -> eyre::Result<()> {
    let interval: std::time::Duration = config
        .interval
        .try_into()
        .wrap_err("metrics interval must be positive")?;

    let client = reqwest::Client::new();

    // Build extra labels query string for Victoria Metrics
    let extra_labels = if config.labels.is_empty() {
        String::new()
    } else {
        let labels: Vec<String> = config
            .labels
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        format!("?extra_label={}", labels.join("&extra_label="))
    };

    let endpoint = format!("{}{}", config.endpoint, extra_labels);

    context.spawn(move |context| async move {
        use commonware_runtime::Clock as _;

        let reth_recorder = install_prometheus_recorder();

        loop {
            context.sleep(interval).await;

            // Collect metrics from both sources
            let consensus_metrics = context.encode();
            let reth_metrics = reth_recorder.handle().render();
            let body = format!("{}\n{}", consensus_metrics, reth_metrics);

            // Push to Victoria Metrics
            let result = client
                .post(&endpoint)
                .header("Content-Type", "text/plain")
                .body(body)
                .send()
                .await;

            if let Err(e) = result {
                tracing::warn!(error = %e, "failed to push metrics");
            }
        }
    });

    Ok(())
}
