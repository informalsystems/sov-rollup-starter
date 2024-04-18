#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use async_trait::async_trait;
use sov_db::ledger_db::LedgerDb;
use sov_kernels::basic::BasicKernel;
use sov_mock_da::{MockDaConfig, MockDaService, MockDaSpec};
use sov_mock_zkvm::{MockCodeCommitment, MockZkvm};
use sov_modules_api::default_spec::{DefaultSpec, ZkDefaultSpec};
use sov_modules_api::{Spec, Zkvm};
use sov_modules_rollup_blueprint::RollupBlueprint;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_prover_storage_manager::ProverStorageManager;
use sov_risc0_adapter::host::Risc0Host;
use sov_rollup_interface::zk::{aggregated_proof::CodeCommitment, ZkvmGuest, ZkvmHost};
use sov_sequencer::SequencerDb;
use sov_state::config::Config as StorageConfig;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ZkStorage};
use sov_stf_runner::RollupConfig;
use sov_stf_runner::RollupProverConfig;
use sov_stf_runner::{ParallelProverService, ProverService};
use stf_starter::Runtime;
use tokio::sync::watch;

/// Rollup with [`MockDaService`].
pub struct MockRollup {}

/// This is the place, where all the rollup components come together, and
/// they can be easily swapped with alternative implementations as needed.
#[async_trait]
impl RollupBlueprint for MockRollup {
    /// This component defines the Data Availability layer.
    type DaService = MockDaService;
    type DaSpec = MockDaSpec;
    type DaConfig = MockDaConfig;

    /// Inner Zkvm representing the rollup circuit
    type InnerZkvmHost = Risc0Host<'static>;
    /// Outer Zkvm representing the circuit verifier for recursion
    type OuterZkvmHost = MockZkvm;

    /// Spec for the Zero Knowledge environment.
    type ZkSpec = ZkDefaultSpec<
        <<Self::InnerZkvmHost as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
        <<Self::OuterZkvmHost as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
    >;
    /// Spec for the Native environment.
    type NativeSpec = DefaultSpec<
        <<Self::InnerZkvmHost as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
        <<Self::OuterZkvmHost as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
    >;

    /// Manager for the native storage lifecycle.
    type StorageManager = ProverStorageManager<MockDaSpec, DefaultStorageSpec>;

    /// Runtime for the Zero Knowledge environment.
    type ZkRuntime = Runtime<Self::ZkSpec, Self::DaSpec>;
    /// Runtime for the Native environment.
    type NativeRuntime = Runtime<Self::NativeSpec, Self::DaSpec>;

    /// Kernels.
    type NativeKernel = BasicKernel<Self::NativeSpec, Self::DaSpec>;
    type ZkKernel = BasicKernel<Self::ZkSpec, Self::DaSpec>;

    /// Prover service.
    type ProverService = ParallelProverService<
        <<Self::NativeSpec as Spec>::Storage as Storage>::Root,
        <<Self::NativeSpec as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        Self::InnerZkvmHost,
        Self::OuterZkvmHost,
        StfBlueprint<
            Self::ZkSpec,
            Self::DaSpec,
            <<Self::InnerZkvmHost as ZkvmHost>::Guest as ZkvmGuest>::Verifier,
            Self::ZkRuntime,
            Self::ZkKernel,
        >,
    >;

    fn create_outer_code_commitment(
        &self,
    ) -> <<Self::ProverService as ProverService>::Verifier as Zkvm>::CodeCommitment {
        MockCodeCommitment::default()
    }

    /// This function generates RPC methods for the rollup, allowing for extension with custom endpoints.
    fn create_endpoints(
        &self,
        storage: watch::Receiver<<Self::NativeSpec as Spec>::Storage>,
        ledger_db: &LedgerDb,
        sequencer_db: &SequencerDb,
        da_service: &Self::DaService,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> Result<(jsonrpsee::RpcModule<()>, axum::Router<()>), anyhow::Error> {
        sov_modules_rollup_blueprint::register_endpoints::<Self>(
            storage.clone(),
            ledger_db,
            sequencer_db,
            da_service,
            rollup_config.da.sender_address,
        )
    }

    async fn create_da_service(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> Self::DaService {
        MockDaService::new(rollup_config.da.sender_address)
    }

    async fn create_prover_service(
        &self,
        prover_config: RollupProverConfig,
        _rollup_config: &RollupConfig<Self::DaConfig>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let inner_vm = Risc0Host::new(risc0_starter::MOCK_DA_ELF);
        let outer_vm = MockZkvm::new_non_blocking();
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();
        let da_verifier = Default::default();

        ParallelProverService::new_with_default_workers(
            inner_vm,
            outer_vm,
            zk_stf,
            da_verifier,
            prover_config,
            zk_storage,
            CodeCommitment::default(),
        )
    }

    fn create_storage_manager(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> anyhow::Result<Self::StorageManager> {
        let storage_config = StorageConfig {
            path: rollup_config.storage.path.clone(),
        };
        ProverStorageManager::new(storage_config)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint for MockRollup {}
