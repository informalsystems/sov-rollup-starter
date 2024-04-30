//! The demo-rollup supports `EVM` and `sov-module` authenticators.
use std::marker::PhantomData;

use sov_modules_api::runtime::capabilities::{AuthenticationError, RawTx};
use sov_modules_api::transaction::AuthenticatedTransactionAndRawHash;
use sov_modules_api::{Authenticator, DaSpec, DispatchCall, Spec};

use crate::runtime::Runtime;

pub struct ModAuth<S: Spec, Da: DaSpec> {
    _phantom: PhantomData<(S, Da)>,
}

impl<S: Spec, Da: DaSpec> Authenticator for ModAuth<S, Da> {
    type Spec = S;
    type DispatchCall = Runtime<S, Da>;
    fn authenticate(
        tx: &[u8],
    ) -> Result<
        (
            AuthenticatedTransactionAndRawHash<Self::Spec>,
            <Self::DispatchCall as DispatchCall>::Decodable,
        ),
        AuthenticationError,
    > {
        sov_modules_api::authenticate::<Self::Spec, Self::DispatchCall>(tx)
    }

    fn encode(tx: Vec<u8>) -> Result<RawTx, anyhow::Error> {
        Ok(RawTx { data: tx })
    }
}
