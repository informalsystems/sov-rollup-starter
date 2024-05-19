//! This module implements the various "hooks" that are called by the STF during execution.
//! These hooks can be used to add custom logic at various points in the slot lifecycle:
//! - Before and after each transaction is executed.
//! - At the beginning and end of each batch ("blob")
//! - At the beginning and end of each slot (DA layer block)

use sov_bank::IntoPayable;
use sov_modules_api::batch::BatchWithId;
use sov_modules_api::hooks::{ApplyBatchHooks, FinalizeHook, SlotHooks, TxHooks};
use sov_modules_api::namespaces::Accessory;
use sov_modules_api::runtime::capabilities::{
    AuthenticationError, GasEnforcer, RawTx, RuntimeAuthenticator, RuntimeAuthorization,
};
use sov_modules_api::transaction::{
    AuthenticatedTransactionAndRawHash, AuthenticatedTransactionData,
};
use sov_modules_api::{
    Context, DaSpec, DispatchCall, Gas, ModuleInfo, Spec, StateCheckpoint, StateReaderAndWriter,
    WorkingSet,
};
use sov_modules_stf_blueprint::SequencerOutcome;
use sov_sequencer_registry::SequencerRegistry;

use super::runtime::Runtime;

impl<S: Spec, Da: DaSpec> GasEnforcer<S, Da> for Runtime<S, Da> {
    /// The transaction type that the gas enforcer knows how to parse
    type Tx = AuthenticatedTransactionData<S>;
    /// Reserves enough gas for the transaction to be processed, if possible.
    fn try_reserve_gas(
        &self,
        tx: &Self::Tx,
        context: &Context<S>,
        gas_price: &<S::Gas as Gas>::Price,
        mut state_checkpoint: StateCheckpoint<S>,
    ) -> Result<WorkingSet<S>, StateCheckpoint<S>> {
        match self
            .bank
            .reserve_gas(tx, gas_price, context.sender(), &mut state_checkpoint)
        {
            Ok(gas_meter) => Ok(state_checkpoint.to_revertable(gas_meter)),
            Err(e) => {
                tracing::debug!("Unable to reserve gas from {}. {}", e, context.sender());
                Err(state_checkpoint)
            }
        }
    }

    /// Refunds any remaining gas to the payer after the transaction is processed.
    fn refund_remaining_gas(
        &self,
        tx: &Self::Tx,
        context: &Context<S>,
        gas_meter: &sov_modules_api::GasMeter<S::Gas>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.bank.refund_remaining_gas(
            tx,
            gas_meter,
            context.sender(),
            &self.prover_incentives.id().to_payable(),
            &self.sequencer_registry.id().to_payable(),
            state_checkpoint,
        );
    }
}

impl<S: Spec, Da: DaSpec> RuntimeAuthorization<S, Da> for Runtime<S, Da> {
    /// Resolves the context for a transaction.
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
            .ok_or(anyhow::anyhow!("Sequencer was no longer registered by the time of context resolution. This is a bug")).unwrap();
        let sender = self.accounts.resolve_sender_address(tx, state_checkpoint)?;
        Ok(Context::new(sender, sequencer, height))
    }

    /// Prevents duplicate transactions from running.
    // TODO(@preston-evans98): Use type system to prevent writing to the `StateCheckpoint` during this check
    fn check_uniqueness(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        _context: &Context<S>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> Result<(), anyhow::Error> {
        self.accounts.check_uniqueness(tx, state_checkpoint)
    }

    /// Marks a transaction as having been executed, preventing it from executing again.
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

    fn pre_dispatch_tx_hook(
        &self,
        _tx: &AuthenticatedTransactionData<Self::Spec>,
        _working_set: &mut WorkingSet<S>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn post_dispatch_tx_hook(
        &self,
        _tx: &AuthenticatedTransactionData<Self::Spec>,
        _ctx: &Context<S>,
        _working_set: &mut WorkingSet<S>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: Spec, Da: DaSpec> ApplyBatchHooks<Da> for Runtime<S, Da> {
    type Spec = S;
    type BatchResult = SequencerOutcome;

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
        // Since we need to make sure the `StfBlueprint`
        // doesn't depend on the module system, we need to
        // convert the `SequencerOutcome` structures manually.
        match result {
            SequencerOutcome::Rewarded(amount) => {
                tracing::info!(%sender, ?amount, "Rewarding sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Rewarded(amount),
                    sender,
                    state_checkpoint,
                );
            }
            SequencerOutcome::Ignored => {}
            SequencerOutcome::Slashed(reason) => {
                tracing::info!(%sender, ?reason, "Slashing sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Slashed,
                    sender,
                    state_checkpoint,
                );
            }
            SequencerOutcome::Penalized(amount) => {
                tracing::info!(amount, "Penalizing sequencer");
                <SequencerRegistry<S, Da> as ApplyBatchHooks<Da>>::end_batch_hook(
                    &self.sequencer_registry,
                    sov_sequencer_registry::SequencerOutcome::Penalized(amount),
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
        _pre_state_root: <Self::Spec as Spec>::VisibleHash,
        _versioned_working_set: &mut sov_modules_api::VersionedStateReadWriter<
            StateCheckpoint<Self::Spec>,
        >,
    ) {
    }

    fn end_slot_hook(&self, _working_set: &mut StateCheckpoint<S>) {}
}

impl<S: Spec, Da: DaSpec> FinalizeHook for Runtime<S, Da> {
    type Spec = S;

    fn finalize_hook(
        &self,
        _root_hash: S::VisibleHash,
        _accessory_working_set: &mut impl StateReaderAndWriter<Accessory>,
    ) {
    }
}

impl<S: Spec, Da: DaSpec> RuntimeAuthenticator for Runtime<S, Da> {
    type Decodable = <Self as DispatchCall>::Decodable;

    type Tx = AuthenticatedTransactionAndRawHash<S>;

    #[cfg_attr(all(target_os = "zkvm", feature = "bench"), cycle_tracker)]
    fn authenticate(
        &self,
        raw_tx: &RawTx,
    ) -> Result<(Self::Tx, Self::Decodable), AuthenticationError> {
        sov_modules_api::authenticate::<S, Self>(&raw_tx.data)
    }
}
