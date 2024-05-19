//! The stf-rollup supports `sov-module` authenticator. To support other authentication schemes,
//! you can check out how we support `EVM` authenticator here:
//! https://github.com/Sovereign-Labs/sovereign-sdk-wip/blob/146d5c2c5fa07ab7bb59ba6b2e64690ac9b63830/examples/demo-rollup/stf/src/authentication.rs#L29-L32
use std::marker::PhantomData;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use sov_modules_api::runtime::capabilities::{AuthenticationError, RawTx};
use sov_modules_api::transaction::AuthenticatedTransactionAndRawHash;
use sov_modules_api::{Authenticator, DaSpec, DispatchCall, GasMeter, Spec};
use sov_modules_api::capabilities::{FatalError, RuntimeAuthenticator};
use sov_sequencer_registry::SequencerStakeMeter;

use crate::runtime::Runtime;

impl<S: Spec, Da: DaSpec> RuntimeAuthenticator<S> for Runtime<S, Da> {
    type Decodable = <Self as DispatchCall>::Decodable;

    type SequencerStakeMeter = SequencerStakeMeter<S::Gas>;

    #[cfg_attr(all(target_os = "zkvm", feature = "bench"), cycle_tracker)]
    fn authenticate(
        &self,
        raw_tx: &RawTx,
        sequencer_stake_meter: &mut Self::SequencerStakeMeter,
    ) -> Result<(AuthenticatedTransactionAndRawHash<S>, Self::Decodable), AuthenticationError> {
        let auth = Auth::try_from_slice(raw_tx.data.as_slice()).map_err(|e| {
            AuthenticationError::FatalError(FatalError::DeserializationFailed(e.to_string()))
        })?;

        match auth {
            Auth::Mod(tx) => ModAuth::<S, Da>::authenticate(&tx, sequencer_stake_meter),
            // Leaving the line below as an example to support different authentication schemes:
            // Auth::Evm(tx) => EvmAuth::<S, Da>::authenticate(&tx, sequencer_stake_meter),
        }
    }
}

#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
enum Auth {
    Mod(Vec<u8>),
    // Leaving the line below as an example to support different authentication schemes:
    // Evm(Vec<u8>),
}

pub struct ModAuth<S: Spec, Da: DaSpec> {
    _phantom: PhantomData<(S, Da)>,
}

impl<S: Spec, Da: DaSpec> Authenticator for ModAuth<S, Da> {
    type Spec = S;
    type DispatchCall = Runtime<S, Da>;
    fn authenticate(
        tx: &[u8],
        stake_meter: &mut impl GasMeter<S::Gas>,
    ) -> Result<
        (
            AuthenticatedTransactionAndRawHash<Self::Spec>,
            <Self::DispatchCall as DispatchCall>::Decodable,
        ),
        AuthenticationError,
    > {
        sov_modules_api::authenticate::<Self::Spec, Self::DispatchCall>(tx, stake_meter)
    }

    fn encode(tx: Vec<u8>) -> Result<RawTx, anyhow::Error> {
        let data = Auth::Mod(tx).try_to_vec()?;
        Ok(RawTx { data })
    }
}
