use bytes::{Buf, BufMut};
use commonware_codec::{Error, Read, ReadExt as _, Write};
use commonware_consensus::simplex::{
    signing_scheme::Scheme,
    types::{Finalization, Notarization},
};
use commonware_cryptography::Digest;
use reth_revm::primitives::hardfork::SpecId::SHANGHAI;

use crate::consensus::block::Block;

pub struct Notarized<S: Scheme, D: Digest> {
    pub block: Block,
    pub notarization: Notarization<S, D>,
}

pub struct Finalized<S: Scheme, D: Digest> {
    pub block: Block,
    pub finalization: Finalization<S, D>,
}

impl<S: Scheme, D: Digest> Write for Finalized<S, D> {
    fn write(&self, buf: &mut impl BufMut) {
        self.block.write(buf);
        self.finalization.write(buf);
    }
}

impl<S: Scheme, D: Digest> Write for Notarized<S, D> {
    fn write(&self, buf: &mut impl BufMut) {
        self.block.write(buf);
        self.notarization.write(buf);
    }
}

impl<S: Scheme, D: Digest> Read for Finalized<S, D> {
    type Cfg = <S::Certificate as Read>::Cfg;

    fn read_cfg(buf: &mut impl Buf, cfg: &Self::Cfg) -> Result<Self, Error> {
        let block = Block::read(buf)?;
        let finalization = Finalization::<S, D>::read_cfg(buf, cfg)?;

        if finalization.proposal.payload != block.digest() {
            return Err(Error::Invalid(
                "types::finalized",
                "Proof payload does not match block digest",
            ));
        }

        Ok(Finalized {
            block,
            finalization,
        })
    }
}
