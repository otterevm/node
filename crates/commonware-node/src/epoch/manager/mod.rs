mod actor;
pub(super) mod ingress;

use std::{net::SocketAddr, time::Duration};

pub(crate) use actor::Actor;
use commonware_cryptography::{bls12381::primitives::variant::MinSig, ed25519::PublicKey};
use commonware_utils::set::OrderedAssociated;
pub(crate) use ingress::Mailbox;

use commonware_consensus::{marshal, simplex::signing_scheme::bls12381_threshold::Scheme};
use commonware_p2p::Blocker;
use commonware_runtime::{Clock, Metrics, Network, Spawner, Storage, buffer::PoolRef};
use rand::{CryptoRng, Rng};

use crate::{consensus::block::Block, epoch::scheme_provider::SchemeProvider, subblocks};

pub(crate) struct Config<TBlocker, TPeerManager> {
    pub(crate) application: crate::consensus::application::Mailbox,
    pub(crate) blocker: TBlocker,
    pub(crate) peer_manager: TPeerManager,
    pub(crate) buffer_pool: PoolRef,
    pub(crate) epoch_length: u64,
    pub(crate) time_for_peer_response: Duration,
    pub(crate) time_to_propose: Duration,
    pub(crate) mailbox_size: usize,
    pub(crate) subblocks: subblocks::Mailbox,
    pub(crate) marshal: marshal::Mailbox<Scheme<PublicKey, MinSig>, Block>,
    pub(crate) scheme_provider: SchemeProvider,
    pub(crate) time_to_collect_notarizations: Duration,
    pub(crate) time_to_retry_nullify_broadcast: Duration,
    pub(crate) partition_prefix: String,
    pub(crate) views_to_track: u64,
    pub(crate) views_until_leader_skip: u64,
}

pub(crate) fn init<TBlocker, TPeerManager, TContext>(
    config: Config<TBlocker, TPeerManager>,
    context: TContext,
) -> (Actor<TBlocker, TPeerManager, TContext>, Mailbox)
where
    TBlocker: Blocker<PublicKey = PublicKey>,
    TPeerManager: commonware_p2p::Manager<
            PublicKey = PublicKey,
            Peers = OrderedAssociated<PublicKey, SocketAddr>,
        >,
    TContext:
        Spawner + Metrics + Rng + CryptoRng + Clock + governor::clock::Clock + Storage + Network,
{
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let actor = Actor::new(config, context, rx);
    let mailbox = Mailbox::new(tx);
    (actor, mailbox)
}
