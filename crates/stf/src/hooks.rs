//! This module implements the various "hooks" that are called by the STF during execution.
//! These hooks can be used to add custom logic at various points in the slot lifecycle:
//! - Before and after each transaction is executed.
//! - At the beginning and end of each batch ("blob")
//! - At the beginning and end of each slot (DA layer block)

use super::runtime::Runtime;
use sov_modules_api::batch::BatchWithId;
use sov_modules_api::capabilities::RuntimeAuthorization;
use sov_modules_api::hooks::{ApplyBatchHooks, FinalizeHook, SlotHooks, TxHooks};
use sov_modules_api::transaction::AuthenticatedTransactionData;
use sov_modules_api::{Context, DaSpec, Spec, StateCheckpoint, StateReaderAndWriter, WorkingSet};
use sov_modules_stf_blueprint::BatchSequencerOutcome;
use sov_sequencer_registry::SequencerRegistry;
use sov_state::Accessory;
use tracing::info;

// impl<S: Spec, Da: DaSpec> GasEnforcer<S, Da> for Runtime<S, Da> {
//     type PreExecChecksMeter = SequencerStakeMeter<S::Gas>;

//     /// Reserves enough gas for the transaction to be processed, if possible.
//     fn try_reserve_gas(
//         &self,
//         tx: &AuthenticatedTransactionData<S>,
//         context: &Context<S>,
//         gas_price: &<S::Gas as Gas>::Price,
//         pre_exec_checks_meter: &Self::PreExecChecksMeter,
//         state_checkpoint: StateCheckpoint<S>,
//     ) -> Result<WorkingSet<S>, StateCheckpoint<S>> {
//         self.bank
//             .reserve_gas(
//                 tx,
//                 gas_price,
//                 context.sender(),
//                 pre_exec_checks_meter,
//                 state_checkpoint,
//             )
//             .map_err(
//                 |ReserveGasError::<S> {
//                      state_checkpoint,
//                      reason,
//                  }| {
//                     tracing::debug!(
//                         sender = %context.sender(),
//                         error = ?reason,
//                         "Unable to reserve gas from sender"
//                     );
//                     state_checkpoint
//                 },
//             )
//     }

//     /// Refunds any remaining gas to the payer after the transaction is processed.
//     fn refund_remaining_gas(
//         &self,
//         tx: &AuthenticatedTransactionData<S>,
//         context: &Context<S>,
//         consumption: &TransactionConsumption<S::Gas>,
//         state_checkpoint: &mut StateCheckpoint<S>,
//     ) {
//         self.bank
//             .refund_remaining_gas(tx, context.sender(), consumption, state_checkpoint);
//     }

//     fn allocate_consumed_gas(
//         &self,
//         tx_consumption: &TransactionConsumption<<S as Spec>::Gas>,
//         checkpoint: &mut StateCheckpoint<S>,
//     ) {
//         // TODO(rano): Implement this
//     }
// }

impl<S: Spec, Da: DaSpec> RuntimeAuthorization<S, Da> for Runtime<S, Da> {
    fn resolve_context(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        sequencer: &Da::Address,
        height: u64,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> Result<Context<S>, anyhow::Error> {
        // TODO(@preston-evans98): This is a temporary hack to get the sequencer address
        // This should be resolved by the sequencer registry during blob selection
        let sequencer = self
            .sequencer_registry
            .resolve_da_address(sequencer, state_checkpoint)
            .ok_or(anyhow::anyhow!("Sequencer was no longer registered by the time of context resolution. This is a bug"))?;
        let sender = self.accounts.resolve_sender_address(tx, state_checkpoint)?;
        // TODO(rano): I don't think, we should use Default here.
        Ok(Context::new(
            sender,
            tx.credentials.clone(),
            sequencer,
            height,
        ))
    }

    fn check_uniqueness(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        _context: &Context<S>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> Result<(), anyhow::Error> {
        self.accounts.check_uniqueness(tx, state_checkpoint)
    }

    fn mark_tx_attempted(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        _sequencer: &Da::Address,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.accounts.mark_tx_attempted(tx, state_checkpoint);
    }
}

impl<S: Spec, Da: DaSpec> TxHooks for Runtime<S, Da> {
    type Spec = S;
    type TxState = WorkingSet<S>;
}

impl<S: Spec, Da: DaSpec> ApplyBatchHooks<Da> for Runtime<S, Da> {
    type Spec = S;
    type BatchResult = BatchSequencerOutcome;

    fn begin_batch_hook(
        &self,
        batch: &mut BatchWithId,
        sender: &Da::Address,
        working_set: &mut StateCheckpoint<S>,
    ) -> anyhow::Result<()> {
        // Before executing each batch, check that the sender is registered as a sequencer
        self.sequencer_registry
            .begin_batch_hook(batch, sender, working_set)
    }

    fn end_batch_hook(
        &self,
        result: Self::BatchResult,
        sender: &Da::Address,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        // Since we need to make sure the `StfBlueprint` doesn't depend on the module system, we need to
        // convert the `SequencerOutcome` structures manually.
        match result {
            BatchSequencerOutcome::Rewarded(amount) => {
                info!(%sender, ?amount, "Rewarding sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Rewarded(amount.into()),
                    sender,
                    state_checkpoint,
                );
            }
            BatchSequencerOutcome::Ignored => {}
            BatchSequencerOutcome::Slashed(reason) => {
                info!(%sender, ?reason, "Slashing sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Slashed,
                    sender,
                    state_checkpoint,
                );
            }
        }
    }
}

impl<S: Spec, Da: DaSpec> SlotHooks for Runtime<S, Da> {
    type Spec = S;

    fn begin_slot_hook(
        &self,
        _pre_state_root: <S as Spec>::VisibleHash,
        _versioned_working_set: &mut sov_modules_api::VersionedStateReadWriter<StateCheckpoint<S>>,
    ) {
    }

    fn end_slot_hook(&self, _working_set: &mut sov_modules_api::StateCheckpoint<S>) {}
}

impl<S: Spec, Da: sov_modules_api::DaSpec> FinalizeHook for Runtime<S, Da> {
    type Spec = S;

    fn finalize_hook(
        &self,
        #[allow(unused_variables)] root_hash: S::VisibleHash,
        #[allow(unused_variables)] accessory_state: &mut impl StateReaderAndWriter<Accessory>,
    ) {
    }
}
