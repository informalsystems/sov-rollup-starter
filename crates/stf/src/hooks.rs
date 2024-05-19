//! This module implements the various "hooks" that are called by the STF during execution.
//! These hooks can be used to add custom logic at various points in the slot lifecycle:
//! - Before and after each transaction is executed.
//! - At the beginning and end of each batch ("blob")
//! - At the beginning and end of each slot (DA layer block)
use anyhow::Context as _;
use sov_bank::IntoPayable;
use sov_modules_api::batch::BatchWithId;
use sov_modules_api::hooks::{ApplyBatchHooks, FinalizeHook, SlotHooks, TxHooks};
use sov_modules_api::runtime::capabilities::{
    GasEnforcer, RuntimeAuthorization, SequencerAuthorization
};
use sov_modules_api::transaction::{AuthenticatedTransactionData, TransactionConsumption, TxGasMeter};
use sov_modules_api::{
    Context, DaSpec, Gas, ModuleInfo, Spec, StateCheckpoint, StateReaderAndWriter,
    WorkingSet,
};
use sov_modules_stf_blueprint::BatchSequencerOutcome;
use sov_sequencer_registry::{SequencerRegistry, SequencerStakeMeter};
use sov_state::namespaces::Accessory;
use tracing::info;

use super::runtime::Runtime;

impl<S: Spec, Da: DaSpec> GasEnforcer<S, Da> for Runtime<S, Da> {
    /// A gas meter that tracks pre-execution costs.
    type PreExecChecksMeter = SequencerStakeMeter<S::Gas>;

    /// Reserves enough gas for the transaction to be processed, if possible.
    fn try_reserve_gas(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        context: &Context<S>,
        gas_price: &<S::Gas as Gas>::Price,
        pre_exec_checks_meter: &SequencerStakeMeter<S::Gas>,
        mut state_checkpoint: StateCheckpoint<S>,
    ) -> Result<WorkingSet<S>, StateCheckpoint<S>> {
        match self.bank.reserve_gas(
            tx,
            gas_price,
            context.sender(),
            pre_exec_checks_meter,
            &mut state_checkpoint,
        ) {
            Ok(gas_meter) => Ok(state_checkpoint.to_revertable(gas_meter)),
            Err(e) => {
                tracing::debug!(
                    sender = %context.sender(),
                    error = ?e,
                    "Unable to reserve gas from sender"
                );
                Err(state_checkpoint)
            }
        }
    }

    fn consume_gas_and_allocate_rewards(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        gas_meter: TxGasMeter<S::Gas>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> TransactionConsumption {
        self.bank.consume_gas_and_allocate_rewards(
            tx,
            gas_meter,
            &self.prover_incentives.id().to_payable(),
            &self.sequencer_registry.id().to_payable(),
            state_checkpoint,
        )
    }

    fn refund_remaining_gas(
        &self,
        tx: &AuthenticatedTransactionData<S>,
        context: &Context<S>,
        consumption: &TransactionConsumption,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.bank
            .refund_remaining_gas(tx, context.sender(), consumption, state_checkpoint);
    }
}

impl<S: Spec, Da: DaSpec> SequencerAuthorization<S, Da> for Runtime<S, Da> {
    type SequencerStakeMeter = SequencerStakeMeter<S::Gas>;

    fn authorize_sequencer(
        &self,
        sequencer: &<Da as DaSpec>::Address,
        base_fee_per_gas: &<S::Gas as Gas>::Price,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> Result<SequencerStakeMeter<S::Gas>, anyhow::Error> {
        self.sequencer_registry
            .authorize_sequencer(sequencer, base_fee_per_gas, state_checkpoint)
            .context("An error occurred while checking the sequencer bond")
    }

    fn refund_sequencer(
        &self,
        sequencer_stake_meter: &mut Self::SequencerStakeMeter,
        refund_amount: u64,
    ) {
        self.sequencer_registry
            .refund_sequencer(sequencer_stake_meter, refund_amount);
    }

    fn penalize_sequencer(
        &self,
        sequencer: &Da::Address,
        sequencer_stake_meter: SequencerStakeMeter<S::Gas>,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) {
        self.sequencer_registry.penalize_sequencer(
            sequencer,
            sequencer_stake_meter,
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
}

impl<S: Spec, Da: DaSpec> ApplyBatchHooks<Da> for Runtime<S, Da> {
    type Spec = S;
    type BatchResult = BatchSequencerOutcome;

    fn begin_batch_hook(
        &self,
        batch: &mut BatchWithId,
        sender: &Da::Address,
        state_checkpoint: &mut StateCheckpoint<S>,
    ) -> anyhow::Result<()> {
        // Before executing each batch, check that the sender is registered as a sequencer
        self.sequencer_registry
            .begin_batch_hook(batch, sender, state_checkpoint)
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
        _pre_state_root: <Self::Spec as Spec>::VisibleHash,
        _versioned_state_checkpoint: &mut sov_modules_api::VersionedStateReadWriter<
            StateCheckpoint<Self::Spec>,
        >,
    ) {}

    fn end_slot_hook(&self, _state_checkpoint: &mut StateCheckpoint<S>) {}
}

impl<S: Spec, Da: DaSpec> FinalizeHook for Runtime<S, Da> {
    type Spec = S;

    fn finalize_hook(
        &self,
        _root_hash: S::VisibleHash,
        _accessory_working_set: &mut impl StateReaderAndWriter<Accessory>,
    ) {}
}
