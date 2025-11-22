use commonware_codec::Write;
use commonware_consensus::simplex::{
    signing_scheme::bls12381_threshold::{Scheme, Seedable},
    types::{Activity, Finalization, Notarization},
};
use commonware_cryptography::{bls12381::primitives::variant::MinSig, ed25519::PublicKey};
use commonware_runtime::{Metrics, Pacer, Spawner};
use tokio::sync::mpsc;
use tracing::warn;

use crate::{alias::marshal, consensus::Digest, indexer::types::Finalized};

pub(crate) struct Actor<TContext> {
    context: TContext,
    actions_rx: mpsc::UnboundedReceiver<ConsensusMessage>,
    marshal: marshal::Mailbox,
}

type ConsensusMessage = Activity<Scheme<PublicKey, MinSig>, Digest>;

impl<TContext: Spawner + Metrics + Pacer> Actor<TContext> {
    pub(crate) fn new(
        context: TContext,
        actions_rx: mpsc::UnboundedReceiver<ConsensusMessage>,
        marshal: marshal::Mailbox,
    ) -> Self {
        Self {
            context,
            actions_rx,
            marshal,
        }
    }

    pub(crate) async fn run(mut self) {
        while let Some(message) = self.actions_rx.recv().await {
            let result = match message {
                Activity::Finalization(finalization) => {
                    self.process_finalization(finalization).await
                }
                Activity::Notarization(notarization) => {
                    self.process_notarization(notarization).await
                }
                _ => Ok(()),
            };
        }
    }

    pub(crate) async fn process_finalization(
        &self,
        finalization: Finalization<Scheme<PublicKey, MinSig>, Digest>,
    ) {
        self.context.with_label("finalized_seed").spawn({
            let seed = finalization.seed();
            async move {}
        });

        self.context.with_label("finalized_block").spawn({
            let mut marshal = self.marshal.clone();
            move |_| async move {
                let block = marshal
                    .subscribe(Some(finalization.round()), finalization.proposal.payload)
                    .await
                    .await;

                let Ok(block) = block else {
                    warn!("Failed to subscribe to block");
                    return;
                };

                let finalized = Finalized {
                    block,
                    finalization,
                };
            }
        });

        Ok(())
    }

    pub(crate) async fn process_notarization(
        &self,
        notarization: Notarization<Scheme<PublicKey, MinSig>, Digest>,
    ) -> eyre::Result<()> {
        let mut buf = Vec::new();

        notarization.write(&mut buf);
        Ok(())
    }
}
