//! This binary runs the rollup full node.

use anyhow::Context;
use clap::Parser;
#[cfg(feature = "celestia_da")]
use sov_celestia_adapter::CelestiaConfig;
#[cfg(feature = "mock_da")]
use sov_mock_da::MockDaConfig;
use sov_modules_rollup_blueprint::{Rollup, RollupBlueprint};
use sov_modules_stf_blueprint::kernels::basic::BasicKernelGenesisConfig;
use sov_modules_stf_blueprint::kernels::basic::BasicKernelGenesisPaths;
#[cfg(feature = "celestia_da")]
use sov_rollup_starter::celestia_rollup::CelestiaRollup;
#[cfg(feature = "mock_da")]
use sov_rollup_starter::mock_rollup::MockRollup;
use sov_stf_runner::RollupProverConfig;
use sov_stf_runner::{from_toml_path, RollupConfig};
use std::str::FromStr;
use stf_starter::genesis_config::GenesisPaths;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

// config and genesis for mock da
#[cfg(feature = "mock_da")]
const DEFAULT_CONFIG_PATH: &str = "../../rollup_config.toml";
#[cfg(feature = "mock_da")]
const DEFAULT_GENESIS_PATH: &str = "../../test-data/genesis/mock/";
#[cfg(feature = "mock_da")]
const DEFAULT_KERNEL_GENESIS_PATH: &str = "../../test-data/genesis/mock/chain_state.json";



// config and genesis for local docker celestia
#[cfg(feature = "celestia_da")]
const DEFAULT_CONFIG_PATH: &str = "../../celestia_rollup_config.toml";
#[cfg(feature = "celestia_da")]
const DEFAULT_GENESIS_PATH: &str = "../../test-data/genesis/celestia/";
#[cfg(feature = "celestia_da")]
const DEFAULT_KERNEL_GENESIS_PATH: &str = "../../test-data/genesis/celestia/chain_state.json";


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the rollup config.
    #[arg(long, default_value = DEFAULT_CONFIG_PATH)]
    rollup_config_path: String,

    /// The path to the genesis config.
    #[arg(long, default_value = DEFAULT_GENESIS_PATH)]
    genesis_paths: String,
    /// The path to the kernel genesis config.
    #[arg(long, default_value = DEFAULT_KERNEL_GENESIS_PATH)]
    kernel_genesis_paths: String,

    /// The optional path to the log file.
    #[arg(long, default_value = None)]
    log_dir: Option<String>,

    /// The optional path to the log file.
    #[arg(long, default_value_t = 9845)]
    metrics: u64,
}

fn init_logging(log_dir: Option<String>) -> Option<WorkerGuard> {
    let (file_layer, guard) = if let Some(path) = log_dir {
        let file_appender = tracing_appender::rolling::daily(&path, "rollup.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        (Some(
            fmt::layer()
                .with_writer(non_blocking)
        ), Some(guard))
    } else {
        (None,None)
    };

    let stdout_layer = fmt::layer().with_writer(std::io::stdout) ;
    let filter_layer = EnvFilter::from_str("debug,hyper=info,risc0_zkvm=warn,sov_prover_storage_manager=info,jmt=info,sov_celestia_adapter=info").unwrap();
    let subscriber = tracing_subscriber::registry()
        .with(stdout_layer)
        .with(filter_layer);

    if let Some(layer) = file_layer {
        subscriber.with(layer).init();
    } else {
        subscriber.init();
    }
    guard
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _guard = init_logging(args.log_dir);

    let metrics_port = args.metrics;
    let address = format!("127.0.0.1:{}", metrics_port);
    prometheus_exporter::start(address.parse().unwrap()).expect("Could not start prometheus server");

    let rollup_config_path = args.rollup_config_path.as_str();

    let genesis_paths = args.genesis_paths.as_str();
    let kernel_genesis_paths = args.kernel_genesis_paths.as_str();

    let prover_config = if option_env!("CI").is_some() {
        Some(RollupProverConfig::Execute)
    } else if let Some(prover) = option_env!("SOV_PROVER_MODE") {
        match prover {
            "simulate" => Some(RollupProverConfig::Simulate),
            "execute" => Some(RollupProverConfig::Execute),
            "prove" => Some(RollupProverConfig::Prove),
            _ => {
                tracing::warn!(
                    prover_mode = prover,
                    "Unknown sov prover mode, using 'Skip' default"
                );
                Some(RollupProverConfig::Skip)
            }
        }
    } else {
        None
    };

    let rollup = new_rollup(
        &GenesisPaths::from_dir(genesis_paths),
        &BasicKernelGenesisPaths {
            chain_state: kernel_genesis_paths.into(),
        },
        rollup_config_path,
        prover_config,
    )
    .await?;
    rollup.run().await
}

#[cfg(feature = "mock_da")]
async fn new_rollup(
    rt_genesis_paths: &GenesisPaths,
    kernel_genesis_paths: &BasicKernelGenesisPaths,
    rollup_config_path: &str,
    prover_config: Option<RollupProverConfig>,
) -> Result<Rollup<MockRollup>, anyhow::Error> {
    info!("Reading rollup config from {rollup_config_path:?}");

    let rollup_config: RollupConfig<MockDaConfig> =
        from_toml_path(rollup_config_path).context("Failed to read rollup configuration")?;

    let mock_rollup = MockRollup {};

    let kernel_genesis = BasicKernelGenesisConfig {
        chain_state: serde_json::from_str(
            &std::fs::read_to_string(&kernel_genesis_paths.chain_state)
                .context("Failed to read chain state")?,
        )?,
    };

    mock_rollup
        .create_new_rollup(
            rt_genesis_paths,
            kernel_genesis,
            rollup_config,
            prover_config,
        )
        .await
}

#[cfg(feature = "celestia_da")]
async fn new_rollup(
    rt_genesis_paths: &GenesisPaths,
    kernel_genesis_paths: &BasicKernelGenesisPaths,
    rollup_config_path: &str,
    prover_config: Option<RollupProverConfig>
) -> Result<Rollup<CelestiaRollup>, anyhow::Error> {
    info!(
        "Starting celestia rollup with config {}",
        rollup_config_path
    );

    let rollup_config: RollupConfig<CelestiaConfig> =
        from_toml_path(rollup_config_path).context("Failed to read rollup configuration")?;

    let kernel_genesis = BasicKernelGenesisConfig {
        chain_state: serde_json::from_str(
            &std::fs::read_to_string(&kernel_genesis_paths.chain_state)
                .context("Failed to read chain state")?,
        )?,
    };

    let mock_rollup = CelestiaRollup {};
    mock_rollup
        .create_new_rollup(
            rt_genesis_paths,
            kernel_genesis,
            rollup_config,
            prover_config,
        )
        .await
}
