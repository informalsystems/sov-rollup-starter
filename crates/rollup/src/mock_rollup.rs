#![deny(missing_docs)]
//! StarterRollup provides a minimal self-contained rollup implementation

use async_trait::async_trait;
use sov_db::ledger_db::LedgerDB;
use sov_mock_da::{MockDaConfig, MockDaService, MockDaSpec};
use sov_modules_api::default_context::{DefaultContext, ZkDefaultContext};
use sov_modules_api::Spec;
use sov_modules_stf_blueprint::kernels::basic::BasicKernel;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_risc0_adapter::host::Risc0Host;
use sov_rollup_interface::zk::ZkvmHost;
use sov_state::config::Config as StorageConfig;
use sov_state::Storage;
use sov_state::{DefaultStorageSpec, ZkStorage};
use sov_stf_runner::BlockingProver;
use sov_stf_runner::RollupConfig;
use sov_stf_runner::RollupProverConfig;
use stf_starter::Runtime;
/// Rollup with [`MockDaService`].

pub struct MockRollup {}

/// This is the place, where all the rollup components come together and
/// they can be easily swapped with alternative implementations as needed.
#[async_trait]
impl sov_modules_rollup_blueprint::RollupBlueprint for MockRollup {
    /// This component defines the Data Availability layer.
    type DaService = MockDaService;
    /// DaSpec & DaConfig are derived from DaService.
    type DaSpec = MockDaSpec;
    type DaConfig = MockDaConfig;

    /// The concrete ZkVm used in the rollup.
    type Vm = Risc0Host<'static>;

    /// Context for the Zero Knowledge environment.
    type ZkContext = ZkDefaultContext;
    /// Context for the ZNative environment.
    type NativeContext = DefaultContext;

    /// Manager for the native storage lifecycle.
    type StorageManager = sov_state::storage_manager::ProverStorageManager<DefaultStorageSpec>;

    /// Runtime for the Zero Knowledge environment.
    type ZkRuntime = Runtime<Self::ZkContext, Self::DaSpec>;
    /// Runtime for the Native environment.
    type NativeRuntime = Runtime<Self::NativeContext, Self::DaSpec>;
    /// Kernels.
    type NativeKernel = BasicKernel<Self::NativeContext>;
    type ZkKernel = BasicKernel<Self::ZkContext>;

    /// Prover service.
    type ProverService = BlockingProver<
        <<Self::NativeContext as Spec>::Storage as Storage>::Root,
        <<Self::NativeContext as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        Self::Vm,
        StfBlueprint<
            Self::ZkContext,
            Self::DaSpec,
            <Self::Vm as ZkvmHost>::Guest,
            Self::ZkRuntime,
            Self::ZkKernel,
        >,
    >;

    /// This function generates RPC methods for the rollup, allowing for extension with custom endpoints.
    fn create_rpc_methods(
        &self,
        storage: &<Self::NativeContext as Spec>::Storage,
        ledger_db: &LedgerDB,
        da_service: &Self::DaService,
    ) -> anyhow::Result<jsonrpsee::RpcModule<()>> {
        sov_modules_rollup_blueprint::register_rpc::<
            Self::NativeRuntime,
            Self::NativeContext,
            Self::DaService,
        >(storage, ledger_db, da_service)
    }

    // Below, we provide the methods for setting up dependencies for the Rollup.

    async fn create_da_service(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> Self::DaService {
        MockDaService::new(rollup_config.da.sender_address)
    }

    fn create_storage_manager(
        &self,
        rollup_config: &RollupConfig<Self::DaConfig>,
    ) -> anyhow::Result<Self::StorageManager> {
        let storage_config = StorageConfig {
            path: rollup_config.storage.path.clone(),
        };
        sov_state::storage_manager::ProverStorageManager::new(storage_config)
    }

    async fn create_prover_service(
        &self,

        prover_config: Option<RollupProverConfig>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let vm = Risc0Host::new(risc0_starter::MOCK_DA_ELF);
        let zk_stf = StfBlueprint::new();
        let zk_storage = ZkStorage::new();
        let da_verifier = Default::default();

        BlockingProver::new(vm, zk_stf, da_verifier, prover_config, zk_storage)
    }
}

impl sov_modules_rollup_blueprint::WalletBlueprint for MockRollup {}