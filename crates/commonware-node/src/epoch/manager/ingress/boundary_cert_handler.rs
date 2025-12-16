use std::fmt::Display;

use bytes::Bytes;
use commonware_codec::{EncodeSize, Read, Write, varint::UInt};
use commonware_consensus::types::Epoch;
use commonware_resolver::{Consumer, p2p::Producer};
use futures::{
    SinkExt as _,
    channel::{mpsc, oneshot},
};
use tracing::{debug, warn};

pub(in crate::epoch::manager) fn new() -> (FinalizationHandler, mpsc::Receiver<Message>) {
    let (tx, rx) = mpsc::channel(16);
    (FinalizationHandler { inner: tx }, rx)
}

#[derive(Debug)]
pub(in crate::epoch::manager) struct Message {
    pub(in crate::epoch::manager) cause: tracing::Span,
    pub(in crate::epoch::manager) action: Action,
}

impl Message {
    fn in_current_span(action: impl Into<Action>) -> Self {
        Self {
            cause: tracing::Span::current(),
            action: action.into(),
        }
    }
}

#[derive(Debug)]
pub(in crate::epoch::manager) enum Action {
    Deliver(Deliver),
    Produce(Produce),
}

impl From<Deliver> for Action {
    fn from(value: Deliver) -> Self {
        Self::Deliver(value)
    }
}

impl From<Produce> for Action {
    fn from(value: Produce) -> Self {
        Self::Produce(value)
    }
}

#[derive(Debug)]
pub(in crate::epoch::manager) struct Deliver {
    pub(in crate::epoch::manager) epoch: Epoch,
    pub(in crate::epoch::manager) value: Bytes,
    pub(in crate::epoch::manager) response: oneshot::Sender<bool>,
}

#[derive(Debug)]
pub(in crate::epoch::manager) struct Produce {
    pub(in crate::epoch::manager) epoch: Epoch,
    pub(in crate::epoch::manager) response: oneshot::Sender<Bytes>,
}

/// Newtype wrapper to impl [`Span`] until [`Epoch`] implements `Span` natively.
// TODO(janis): remove once https://github.com/commonwarexyz/monorepo/pull/2287 is merged
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::epoch::manager) struct SpanEpoch(pub(in crate::epoch::manager) Epoch);

impl Display for SpanEpoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl commonware_utils::Span for SpanEpoch {}

impl Read for SpanEpoch {
    type Cfg = ();

    fn read_cfg(
        buf: &mut impl bytes::Buf,
        _cfg: &Self::Cfg,
    ) -> Result<Self, commonware_codec::Error> {
        let epoch = UInt::read_cfg(buf, &())?.into();
        Ok(Self(epoch))
    }
}

impl Write for SpanEpoch {
    fn write(&self, buf: &mut impl bytes::BufMut) {
        UInt(self.0).write(buf)
    }
}

impl EncodeSize for SpanEpoch {
    fn encode_size(&self) -> usize {
        UInt(self.0).encode_size()
    }
}

#[derive(Clone)]
pub(in crate::epoch::manager) struct FinalizationHandler {
    inner: mpsc::Sender<Message>,
}

impl Consumer for FinalizationHandler {
    type Key = SpanEpoch;

    type Value = Bytes;

    type Failure = ();

    async fn deliver(&mut self, key: Self::Key, value: Self::Value) -> bool {
        let (response, rx) = oneshot::channel();
        if let Err(error) = self
            .inner
            .send(Message::in_current_span(Deliver {
                epoch: key.0,
                value,
                response,
            }))
            .await
        {
            warn!(error = %eyre::Report::new(error), "failed to send deliver message to actor");
            return false;
        }

        rx.await
            .inspect_err(|_| warn!("response channel was dropped before actor acknowledged it"))
            .unwrap_or(false)
    }

    async fn failed(&mut self, key: Self::Key, _failure: Self::Failure) {
        debug!(epoch = %key, "fetching finalization certificate failed");
    }
}

impl Producer for FinalizationHandler {
    type Key = SpanEpoch;

    async fn produce(&mut self, key: Self::Key) -> oneshot::Receiver<Bytes> {
        let (response, rx) = oneshot::channel();
        if let Err(error) = self
            .inner
            .send(Message::in_current_span(Produce {
                epoch: key.0,
                response,
            }))
            .await
        {
            warn!(error = %eyre::Report::new(error), "failed to send produce message to actor");
        }
        rx
    }
}
