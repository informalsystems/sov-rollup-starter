use anyhow::Context;
use stf_starter::runtime::RuntimeCall;
use sov_modules_api::transaction::Transaction;
use sov_celestia_adapter::verifier::CelestiaSpec;
use sov_risc0_adapter::Risc0Verifier;
use sov_modules_api::{PrivateKey, Spec, CryptoSpec, AddressBech32};
use sov_bank::CallMessage;
use borsh::ser::BorshSerialize;
use jsonrpsee::core::client::{ClientT, Subscription, SubscriptionClientT};
use jsonrpsee::core::Error;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde::Serialize;
use sov_accounts::AccountsRpcClient;
use sov_bank::Coins;
use sov_cli::wallet_state::PrivateKeyAndAddress;
use sov_sequencer::utils::SimpleClient;
use rand::Rng;
use tokio::signal;

pub type TestSpec = sov_modules_api::default_spec::DefaultSpec<Risc0Verifier>;
pub type TestPrivateKey = <<TestSpec as Spec>::CryptoSpec as CryptoSpec>::PrivateKey;

const TOKEN_ADDRESS: &'static str = "sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72";
const RPC_HOST: &'static str = "54.202.215.214";
const RPC_PORT: u16 = 12345;

async fn get_nonce_for_account<S: Spec + Send + Sync + Serialize>(
    client:  &(impl ClientT + Send + Sync),
    account_pubkey: &<S::CryptoSpec as CryptoSpec>::PublicKey,
) -> Result<u64, anyhow::Error> {
    Ok(match AccountsRpcClient::<S>::get_account(
        client,
        account_pubkey.clone(),
    )
        .await
        .context(
            "Unable to connect to provided RPC. You can change to a different RPC url with the `rpc set-url` subcommand ",
        )? {
        sov_accounts::Response::AccountExists { addr: _, nonce } => nonce,
        _ => 0,
    })
}

async fn get_balance(
    client: &SimpleClient,
    user_address: <TestSpec as Spec>::Address,
) -> Result<Option<u64>, Error> {
    let balance_response = sov_bank::BankRpcClient::<TestSpec>::balance_of(
        client.http(),
        None,
        user_address,
        AddressBech32::try_from(TOKEN_ADDRESS.to_string()).unwrap().into(),
    )
        .await;
    balance_response.map(|x|x.amount)
}

async fn submit_transfer(client: &SimpleClient, private_key: &TestPrivateKey) -> anyhow::Result<u64> {
    let http_client = HttpClientBuilder::default().build(format!("http://{}:{}",RPC_HOST,RPC_PORT))?;
    let current_nonce = get_nonce_for_account::<TestSpec>(&http_client, &private_key.pub_key()).await?;
    let start_balance = get_balance(&client, private_key.to_address()).await?;
    let balance = start_balance.context("Couldn't get balance")?;
    let mut rng = rand::thread_rng();
    let number: u64 = rng.gen_range(1..=20);
    let new_expected_balance = balance - number;
    let mut txns = vec![];
    for n in 0..number {
        let recipient_key = PrivateKeyAndAddress::<TestSpec>::generate();
        txns.push(build_transfer_token_tx(&private_key, recipient_key.address, current_nonce + n));
    }

    send_transactions_and_wait_slot(client, txns).await?;

    subscribe_wait_n_slots(client,2).await?;

    let start_balance = get_balance(&client, private_key.to_address()).await?;
    let balance = start_balance.context("Couldn't get balance")?;
    assert_eq!(new_expected_balance,balance);

    Ok(number)
}

async fn subscribe_wait_n_slots(client: &SimpleClient, n: usize) -> Result<(), anyhow::Error> {
    let mut slot_subscription: Subscription<u64> = client
        .ws()
        .subscribe(
            "ledger_subscribeSlots",
            rpc_params![],
            "ledger_unsubscribeSlots",
        )
        .await?;
    for _ in 0..n {
        let _ = slot_subscription.next().await;
    }
    Ok(())
}

async fn send_transactions_and_wait_slot(
    client: &SimpleClient,
    transactions: Vec<Transaction<TestSpec>>,
) -> Result<(), anyhow::Error> {
    let mut slot_subscription: Subscription<u64> = client
        .ws()
        .subscribe(
            "ledger_subscribeSlots",
            rpc_params![],
            "ledger_unsubscribeSlots",
        )
        .await?;

    client.send_transactions(transactions, None).await?;
    let _ = slot_subscription.next().await;

    Ok(())
}

fn build_transfer_token_tx(
    key: &TestPrivateKey,
    recipient: <TestSpec as Spec>::Address,
    nonce: u64,
) -> Transaction<TestSpec> {
    let msg =
        RuntimeCall::<TestSpec, CelestiaSpec>::bank(CallMessage::<TestSpec>::Transfer {
            to: recipient,
            coins: Coins {
                amount: 1,
                token_address: AddressBech32::try_from(TOKEN_ADDRESS.to_string()).unwrap().into(),
            },
        });
    let chain_id = 0;
    let gas_tip = 0;
    let gas_limit = 0;
    let max_gas_price = None;
    Transaction::<TestSpec>::new_signed_tx(
        key,
        msg.try_to_vec().unwrap(),
        chain_id,
        gas_tip,
        gas_limit,
        max_gas_price,
        nonce,
    )
}


#[tokio::main]
async fn main() -> anyhow::Result<()>{
    let mint_key_data =
        std::fs::read_to_string("test-data/keys/minter_private_key.json")
            .expect("Unable to read file to string");

    let mint_key_address: PrivateKeyAndAddress<TestSpec> = serde_json::from_str(&mint_key_data)
        .unwrap_or_else(|_| {
            panic!(
                "Unable to convert data {} to PrivateKeyAndAddress",
                &mint_key_data
            )
        });


    let client = SimpleClient::new(RPC_HOST, RPC_PORT).await?;

    let ctrl_c_handler = async {
        signal::ctrl_c().await.expect("failed to listen for event");
        println!("Termination signal received, exiting...");
    };

    tokio::select! {
        _ = ctrl_c_handler => {},
        _ = async {
            loop {
                match submit_transfer(&client, &mint_key_address.private_key).await {
                    Ok(num_submitted) => {
                        println!("Submitted {} txns",num_submitted)
                    }
                    Err(e) => {
                        println!("{:?}",e)
                    }
                }
            }
        } => {},
    }

    Ok(())

}
