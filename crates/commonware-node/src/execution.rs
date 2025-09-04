//! Contains definitions to pass to the execution layer / reth.

use reth_chainspec::{ChainSpec, EthereumHardforks, Hardforks};
use reth_consensus::{Consensus, ConsensusError, FullConsensus, HeaderValidator};
use reth_ethereum_primitives::EthPrimitives;
use reth_evm::eth::spec::EthExecutorSpec;
use reth_execution_types::BlockExecutionResult;
use reth_node_builder::{Block, BuilderContext, FullNodeTypes, components::ConsensusBuilder};
use reth_primitives_traits::{SealedBlock, SealedHeader};
use std::sync::Arc;

#[derive(Debug, Clone)]
#[expect(
    dead_code,
    reason = "for now only exists to line up arguments in crate::reth_glue::with_runner_and_components"
)]
pub struct TempoConsensus<C = ChainSpec> {
    chain_spec: Arc<C>,
}

impl<C> TempoConsensus<C> {
    pub fn new(chain_spec: Arc<C>) -> Self {
        Self { chain_spec }
    }
}

impl Default for TempoConsensus {
    fn default() -> Self {
        Self {
            chain_spec: Arc::new(ChainSpec::default()),
        }
    }
}

impl<H, C> HeaderValidator<H> for TempoConsensus<C>
where
    C: std::fmt::Debug + Send + Sync,
{
    fn validate_header(&self, _header: &SealedHeader<H>) -> Result<(), ConsensusError> {
        // For now, return Ok - implement validation logic here
        Ok(())
    }

    fn validate_header_against_parent(
        &self,
        _header: &SealedHeader<H>,
        _parent: &SealedHeader<H>,
    ) -> Result<(), ConsensusError> {
        // For now, return Ok - implement validation logic here
        Ok(())
    }
}

impl<B, C> Consensus<B> for TempoConsensus<C>
where
    B: Block,
    C: std::fmt::Debug + Send + Sync,
{
    type Error = ConsensusError;

    fn validate_body_against_header(
        &self,
        _body: &B::Body,
        _header: &SealedHeader<B::Header>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_block_pre_execution(&self, _block: &SealedBlock<B>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<N, C> FullConsensus<N> for TempoConsensus<C>
where
    N: reth_primitives_traits::NodePrimitives,
    C: std::fmt::Debug + Send + Sync,
{
    fn validate_block_post_execution(
        &self,
        _block: &reth_primitives_traits::RecoveredBlock<N::Block>,
        _result: &BlockExecutionResult<N::Receipt>,
    ) -> Result<(), ConsensusError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct TempoConsensusBuilder;

impl<Node> ConsensusBuilder<Node> for TempoConsensusBuilder
where
    Node: FullNodeTypes<
        Types: reth_node_builder::NodeTypes<
            ChainSpec: Hardforks + EthereumHardforks + EthExecutorSpec,
            Primitives = EthPrimitives,
        >,
    >,
{
    type Consensus = Arc<TempoConsensus<<Node::Types as reth_node_builder::NodeTypes>::ChainSpec>>;

    async fn build_consensus(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Consensus> {
        Ok(Arc::new(TempoConsensus::new(ctx.chain_spec())))
    }
}

impl Default for TempoConsensusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TempoConsensusBuilder {
    pub fn new() -> Self {
        Self
    }
}

// use reth_chainspec::ChainSpec;
// use reth_ethereum_primitives::EthPrimitives;
// use reth_node_builder::{
//     BuilderContext, FullNodeTypes, NodeComponentsBuilder, NodeTypes, PayloadBuilderConfig as _,
//     components::{BasicPayloadServiceBuilder, ComponentsBuilder},
// };
// use reth_node_ethereum::{
//     EthEngineTypes, EthEvmConfig, EthereumAddOns, EthereumEngineValidatorBuilder,
//     EthereumEthApiBuilder, EthereumNetworkBuilder, EthereumPoolBuilder,
// };
// use reth_provider::EthStorage;
// use reth_trie_db::MerklePatriciaTrie;

// pub mod consensus_validator;
// pub mod evm;

// #[derive(Debug, Clone)]
// pub struct Node(());

// impl Node {
//     pub fn new() -> Self {
//         Self(())
//     }
// }

// impl Default for Node {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ExecutorBuilder(());

// impl<N: FullNodeTypes<Types = Node>> reth_node_builder::components::ExecutorBuilder<N>
//     for ExecutorBuilder
// {
//     type EVM = EthEvmConfig<ChainSpec, crate::execution::evm::Factory>;

//     async fn build_evm(self, ctx: &BuilderContext<N>) -> eyre::Result<Self::EVM> {
//         Ok(EthEvmConfig::new_with_evm_factory(
//             ctx.chain_spec(),
//             crate::execution::evm::Factory::default(),
//         )
//         .with_extra_data(ctx.payload_builder_config().extra_data_bytes()))
//     }
// }

// impl NodeTypes for Node {
//     type Primitives = EthPrimitives;
//     type ChainSpec = ChainSpec;
//     type StateCommitment = MerklePatriciaTrie;
//     type Storage = EthStorage;
//     type Payload = EthEngineTypes;
// }

// impl<TNodeTypes> reth_node_builder::Node<TNodeTypes> for Node
// where
//     TNodeTypes: FullNodeTypes<Types = Self>,
// {
//     type ComponentsBuilder = ComponentsBuilder<
//         TNodeTypes,
//         EthereumPoolBuilder,
//         BasicPayloadServiceBuilder<reth_node_ethereum::EthereumPayloadBuilder>,
//         EthereumNetworkBuilder,
//         ExecutorBuilder,
//         consensus_validator::Builder,
//     >;

//     type AddOns = EthereumAddOns<
//         reth_node_builder::NodeAdapter<
//             TNodeTypes,
//             <Self::ComponentsBuilder as NodeComponentsBuilder<TNodeTypes>>::Components,
//         >,
//         EthereumEthApiBuilder,
//         EthereumEngineValidatorBuilder,
//     >;

//     fn components_builder(&self) -> Self::ComponentsBuilder {
//         ComponentsBuilder::default()
//             .node_types::<TNodeTypes>()
//             .pool(EthereumPoolBuilder::default())
//             .executor(ExecutorBuilder::default())
//             .payload(BasicPayloadServiceBuilder::default())
//             .network(EthereumNetworkBuilder::default())
//             .consensus(consensus_validator::Builder::new())
//     }

//     fn add_ons(&self) -> Self::AddOns {
//         EthereumAddOns::default()
//     }
// }
