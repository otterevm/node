//! Contains the deprecated logic to read unecrypted at-rest state.

use std::{net::SocketAddr, num::NonZeroU32};

use commonware_codec::{EncodeSize, RangeCfg, Read, ReadExt, Write};
use commonware_consensus::types::Epoch;
use commonware_cryptography::{
    bls12381::{
        dkg::Output,
        primitives::{group::Share, variant::MinSig},
    },
    ed25519::PublicKey,
    transcript::Summary,
};
use commonware_utils::ordered;

/// The outcome of a DKG ceremony.
#[derive(Clone)]
pub(super) struct State {
    pub(super) epoch: Epoch,
    pub(super) seed: Summary,
    pub(super) output: Output<MinSig, PublicKey>,
    pub(super) share: Option<Share>,
    pub(super) dealers: ordered::Map<PublicKey, SocketAddr>,
    pub(super) players: ordered::Map<PublicKey, SocketAddr>,
    // TODO: should these be in the per-epoch state?
    pub(super) syncers: ordered::Map<PublicKey, SocketAddr>,
    /// Whether this DKG ceremony is a full ceremony (new polynomial) instead of a reshare.
    pub(super) is_full_dkg: bool,
}

impl EncodeSize for State {
    fn encode_size(&self) -> usize {
        self.epoch.encode_size()
            + self.seed.encode_size()
            + self.output.encode_size()
            + self.share.encode_size()
            + self.dealers.encode_size()
            + self.players.encode_size()
            + self.syncers.encode_size()
            + self.is_full_dkg.encode_size()
    }
}

impl Write for State {
    fn write(&self, buf: &mut impl bytes::BufMut) {
        self.epoch.write(buf);
        self.seed.write(buf);
        self.output.write(buf);
        self.share.write(buf);
        self.dealers.write(buf);
        self.players.write(buf);
        self.syncers.write(buf);
        self.is_full_dkg.write(buf);
    }
}

impl Read for State {
    type Cfg = NonZeroU32;

    fn read_cfg(
        buf: &mut impl bytes::Buf,
        cfg: &Self::Cfg,
    ) -> Result<Self, commonware_codec::Error> {
        Ok(Self {
            epoch: ReadExt::read(buf)?,
            seed: ReadExt::read(buf)?,
            output: Read::read_cfg(buf, cfg)?,
            share: ReadExt::read(buf)?,
            dealers: Read::read_cfg(buf, &(RangeCfg::from(1..=(u16::MAX as usize)), (), ()))?,
            players: Read::read_cfg(buf, &(RangeCfg::from(1..=(u16::MAX as usize)), (), ()))?,
            syncers: Read::read_cfg(buf, &(RangeCfg::from(1..=(u16::MAX as usize)), (), ()))?,
            is_full_dkg: ReadExt::read(buf)?,
        })
    }
}
