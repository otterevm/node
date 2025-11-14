//! A testing node that can start and stop both consensus and execution layers.

use crate::execution_runtime::{self, ExecutionNode, ExecutionRuntimeHandle};
use commonware_cryptography::ed25519::PublicKey;
use commonware_p2p::simulated::{Control, Oracle, SocketManager};
use commonware_runtime::{Handle, deterministic::Context};
use eyre::WrapErr as _;
use reth_db::{
    init_db,
    mdbx::{DatabaseArguments, DatabaseEnv},
};
use reth_ethereum::provider::{ProviderFactory, providers::StaticFileProvider};
use reth_node_builder::NodeTypesWithDBAdapter;
use std::{path::PathBuf, sync::Arc};
use tempo_commonware_node::consensus;
use tempo_node::node::TempoNode;
use tracing::debug;

/// A testing node that can start and stop both consensus and execution layers.
pub struct TestingNode {
    /// Unique identifier for this node
    pub uid: String,
    /// Public key of the validator
    pub public_key: PublicKey,
    /// Simulated network oracle for test environments
    pub oracle: Oracle<PublicKey>,
    /// Consensus configuration used to start the consensus engine
    pub consensus_config: consensus::Builder<Control<PublicKey>, Context, SocketManager<PublicKey>>,
    /// Running consensus handle (None if consensus is stopped)
    pub consensus_handle: Option<Handle<eyre::Result<()>>>,
    /// Path to the execution node's data directory
    pub execution_node_datadir: PathBuf,
    /// Running execution node (None if execution is stopped)
    pub execution_node: Option<ExecutionNode>,
    /// Handle to the execution runtime for spawning new execution nodes
    pub execution_runtime: ExecutionRuntimeHandle,
}

impl TestingNode {
    /// Create a new TestingNode without spawning execution or starting consensus.
    ///
    /// Call `start()` to start both consensus and execution.
    pub fn new(
        uid: String,
        public_key: PublicKey,
        oracle: Oracle<PublicKey>,
        consensus_config: consensus::Builder<Control<PublicKey>, Context, SocketManager<PublicKey>>,
        execution_runtime: ExecutionRuntimeHandle,
    ) -> Self {
        let execution_node_datadir = execution_runtime
            .nodes_dir()
            .join(execution_runtime::execution_node_name(&public_key));

        Self {
            uid,
            public_key,
            oracle,
            consensus_config,
            consensus_handle: None,
            execution_node: None,
            execution_node_datadir,
            execution_runtime,
        }
    }

    /// Start both consensus and execution layers.
    ///
    ///
    /// # Panics
    /// Panics if either consensus or execution is already running.
    pub async fn start(&mut self) {
        self.start_execution().await;
        self.start_consensus().await;
    }

    /// Start the execution node and update consensus config to reference it.
    ///
    /// # Panics
    /// Panics if execution node is already running.
    async fn start_execution(&mut self) {
        assert!(
            self.execution_node.is_none(),
            "execution node is already running for {}",
            self.uid
        );

        let execution_node = self
            .execution_runtime
            .spawn_node(&execution_runtime::execution_node_name(&self.public_key))
            .await
            .expect("must be able to spawn execution node");

        // Update consensus config to point to the new execution node
        self.consensus_config.execution_node = execution_node.node.clone();
        self.execution_node = Some(execution_node);
        debug!(%self.uid, "started execution node for testing node");
    }

    /// Start the consensus engine with oracle registration.
    ///
    /// # Panics
    /// Panics if consensus is already running.
    async fn start_consensus(&mut self) {
        assert!(
            self.consensus_handle.is_none(),
            "consensus is already running for {}",
            self.uid
        );
        let engine = self
            .consensus_config
            .clone()
            .try_init()
            .await
            .expect("must be able to start the engine");

        let pending = self
            .oracle
            .control(self.public_key.clone())
            .register(0)
            .await
            .unwrap();
        let recovered = self
            .oracle
            .control(self.public_key.clone())
            .register(1)
            .await
            .unwrap();
        let resolver = self
            .oracle
            .control(self.public_key.clone())
            .register(2)
            .await
            .unwrap();
        let broadcast = self
            .oracle
            .control(self.public_key.clone())
            .register(3)
            .await
            .unwrap();
        let marshal = self
            .oracle
            .control(self.public_key.clone())
            .register(4)
            .await
            .unwrap();
        let dkg = self
            .oracle
            .control(self.public_key.clone())
            .register(5)
            .await
            .unwrap();
        let boundary_certs = self
            .oracle
            .control(self.public_key.clone())
            .register(6)
            .await
            .unwrap();
        let subblocks = self
            .oracle
            .control(self.public_key.clone())
            .register(7)
            .await
            .unwrap();

        let consensus_handle = engine.start(
            pending,
            recovered,
            resolver,
            broadcast,
            marshal,
            dkg,
            boundary_certs,
            subblocks,
        );

        self.consensus_handle = Some(consensus_handle);
        debug!(%self.uid, "started consensus for testing node");
    }

    /// Stop both consensus and execution layers.
    ///
    /// # Panics
    /// Panics if either consensus or execution is not running.
    pub fn stop(&mut self) {
        self.stop_consensus();
        self.stop_execution();
    }

    /// Stop only the consensus engine.
    ///
    /// # Panics
    /// Panics if consensus is not running.
    fn stop_consensus(&mut self) {
        let handle = self.consensus_handle.take().expect(&format!(
            "consensus is not running for {}, cannot stop",
            self.uid
        ));
        handle.abort();
        debug!(%self.uid, "stopped consensus for testing node");
    }

    /// Stop only the execution node.
    ///
    /// This triggers a critical task failure which will cause the execution node's
    /// executor to shutdown.
    ///
    /// # Panics
    /// Panics if execution node is not running.
    fn stop_execution(&mut self) {
        let execution_node = self.execution_node.take().expect(&format!(
            "execution node is not running for {}, cannot stop",
            self.uid
        ));

        let uid = self.uid.clone();
        execution_node
            .node
            .task_executor
            .spawn_critical("testing_node_shutdown", async move {
                panic!("TestingNode {} execution shutdown requested", uid);
            });

        debug!(%self.uid, "stopped execution node for testing node");
    }

    /// Check if both consensus and execution are running
    pub fn is_running(&self) -> bool {
        self.consensus_handle.is_some() && self.execution_node.is_some()
    }

    /// Check if consensus is running
    pub fn is_consensus_running(&self) -> bool {
        self.consensus_handle.is_some()
    }

    /// Check if execution is running
    pub fn is_execution_running(&self) -> bool {
        self.execution_node.is_some()
    }

    /// Get a provider factory for the execution node's data directory.
    ///
    /// This can be called even if the execution node is stopped allowing you to
    /// inspect the execution layer state after shutdown.
    pub fn execution_provider(
        &self,
    ) -> eyre::Result<ProviderFactory<NodeTypesWithDBAdapter<TempoNode, Arc<DatabaseEnv>>>> {
        let db_path = self.execution_node_datadir.join("db");
        let database = Arc::new(
            init_db(&db_path, DatabaseArguments::default())
                .wrap_err("failed to open execution node database")?
                .with_metrics(),
        );

        let static_file_provider =
            StaticFileProvider::read_only(self.execution_node_datadir.join("static_files"), true)
                .wrap_err("failed to open static files")?;

        let provider_factory = ProviderFactory::<NodeTypesWithDBAdapter<TempoNode, _>>::new(
            database,
            execution_runtime::chainspec(),
            static_file_provider,
        )?;

        Ok(provider_factory)
    }
}
