//! Prometheus metrics for the Bridge ExEx.

use reth_metrics::{
    Metrics,
    metrics::{Counter, Histogram},
};

/// Bridge ExEx metrics
#[derive(Metrics, Clone)]
#[metrics(scope = "bridge_exex")]
pub struct BridgeMetrics {
    /// Number of deposits detected from origin chains
    pub deposits_detected: Counter,

    /// Number of deposit signatures submitted
    pub deposits_signed: Counter,

    /// Number of deposits finalized on Tempo
    pub deposits_finalized: Counter,

    /// Number of burns detected on Tempo
    pub burns_detected: Counter,

    /// Number of burns unlocked on origin chains
    pub burns_unlocked: Counter,

    /// Successful signature submissions
    pub signature_submissions_success: Counter,

    /// Failed signature submissions
    pub signature_submissions_failure: Counter,

    /// RPC call latency in seconds
    pub rpc_latency_seconds: Histogram,

    /// Proof generation duration in seconds
    pub proof_generation_duration: Histogram,
}

impl BridgeMetrics {
    /// Record a detected deposit
    #[inline]
    pub fn record_deposit_detected(&self) {
        self.deposits_detected.increment(1);
    }

    /// Record a signed deposit
    #[inline]
    pub fn record_deposit_signed(&self) {
        self.deposits_signed.increment(1);
    }

    /// Record a finalized deposit
    #[inline]
    pub fn record_deposit_finalized(&self) {
        self.deposits_finalized.increment(1);
    }

    /// Record a detected burn
    #[inline]
    pub fn record_burn_detected(&self) {
        self.burns_detected.increment(1);
    }

    /// Record an unlocked burn
    #[inline]
    pub fn record_burn_unlocked(&self) {
        self.burns_unlocked.increment(1);
    }

    /// Record a successful signature submission
    #[inline]
    pub fn record_signature_success(&self) {
        self.signature_submissions_success.increment(1);
    }

    /// Record a failed signature submission
    #[inline]
    pub fn record_signature_failure(&self) {
        self.signature_submissions_failure.increment(1);
    }

    /// Record RPC latency
    #[inline]
    pub fn record_rpc_latency(&self, duration_secs: f64) {
        self.rpc_latency_seconds.record(duration_secs);
    }

    /// Record proof generation duration
    #[inline]
    pub fn record_proof_generation(&self, duration_secs: f64) {
        self.proof_generation_duration.record(duration_secs);
    }
}
