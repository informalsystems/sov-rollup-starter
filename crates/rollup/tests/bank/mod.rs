use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

use super::test_helpers::{read_private_keys, start_rollup};
use borsh::BorshSerialize;
use jsonrpsee::core::client::{Subscription, SubscriptionClientT};
use jsonrpsee::rpc_params;
use sov_kernels::basic::BasicKernelGenesisPaths;
use sov_mock_da::{MockAddress, MockDaConfig, MockDaSpec};
use sov_modules_api::transaction::{PriorityFeeBips, Transaction, UnsignedTransaction};
use sov_modules_api::Spec;
use sov_sequencer::utils::SimpleClient;
use sov_stf_runner::RollupProverConfig;
use stf_starter::genesis_config::GenesisPaths;
use stf_starter::RuntimeCall;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

const TOKEN_SALT: u64 = 0;
const TOKEN_NAME: &str = "sov-token";
const MAX_TX_FEE: u64 = 10_000;

type TestSpec = sov_modules_api::default_spec::DefaultSpec<
    sov_mock_zkvm::MockZkVerifier,
    sov_mock_zkvm::MockZkVerifier,
>;

#[tokio::test]
async fn bank_tx_tests() -> Result<(), anyhow::Error> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_str(
                &env::var("RUST_LOG")
                    .unwrap_or_else(|_| "debug,hyper=info,jmt=info,risc0_zkvm=info".to_string()),
            )
            .unwrap(),
        )
        .init();
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();

    let rollup_task = tokio::spawn(async {
        start_rollup(
            port_tx,
            GenesisPaths::from_dir("../../test-data/genesis/mock/"),
            BasicKernelGenesisPaths {
                chain_state: "../../test-data/genesis/mock/chain_state.json".into(),
            },
            RollupProverConfig::Skip,
            MockDaConfig {
                sender_address: MockAddress::new([0; 32]),
                finalization_blocks: 3,
                wait_attempts: 10,
            },
        )
        .await;
    });
    let port = port_rx.await.unwrap();

    // If the rollup throws an error, return it and stop trying to send the transaction
    tokio::select! {
        err = rollup_task => err?,
        res = send_test_create_token_tx(port) => res?,
    }
    Ok(())
}

async fn send_test_create_token_tx(rpc_address: SocketAddr) -> Result<(), anyhow::Error> {
    let key_and_address = read_private_keys::<TestSpec>("tx_signer_private_key.json");
    let key = key_and_address.private_key;
    let user_address: <TestSpec as Spec>::Address = key_and_address.address;

    let token_id = sov_bank::get_token_id::<TestSpec>(TOKEN_NAME, &user_address, TOKEN_SALT);
    let initial_balance = 1000;

    let msg =
        RuntimeCall::<TestSpec, MockDaSpec>::bank(sov_bank::CallMessage::<TestSpec>::CreateToken {
            salt: TOKEN_SALT,
            token_name: TOKEN_NAME.to_string(),
            initial_balance,
            minter_address: user_address,
            authorized_minters: vec![],
        });
    let chain_id = 0;
    let nonce = 0;
    let max_priority_fee = PriorityFeeBips::ZERO;
    let gas_limit = None;
    let tx = Transaction::<TestSpec>::new_signed_tx(
        &key,
        UnsignedTransaction::new(
            msg.try_to_vec().unwrap(),
            chain_id,
            max_priority_fee,
            MAX_TX_FEE,
            nonce,
            gas_limit,
        ),
    );

    let port = rpc_address.port();
    let client = SimpleClient::new("localhost", port).await?;

    let mut slot_processed_subscription: Subscription<u64> = client
        .ws()
        .subscribe(
            "ledger_subscribeSlots",
            rpc_params![],
            "ledger_unsubscribeSlots",
        )
        .await?;

    client.send_transactions(&[tx]).await.unwrap();
    // Wait until the rollup has processed the next slot
    let _ = slot_processed_subscription.next().await;

    let balance_response = sov_bank::BankRpcClient::<TestSpec>::balance_of(
        client.http(),
        None,
        user_address,
        token_id,
    )
    .await?;
    assert_eq!(
        initial_balance,
        balance_response.amount.unwrap_or_default(),
        "deployer initial balance is not correct"
    );
    Ok(())
}
