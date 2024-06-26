This package is a convenient starting point for building a rollup using the Sovereign SDK:

# The repo structure:
- `crates/stf`:  The `STF` is derived from the `Runtime` and is used in the `rollup` and `provers` crates.
- `crates/provers`: This crate is responsible for creating proofs for the `STF`.
- `crates/rollup`: This crate runs the `STF` and offers additional full-node functionalities.

(!) Note for using WIP repo.
This repo utilizes private Sovereign SDK repo and default cargo needs this environment variable to use ssh key:

```
export CARGO_NET_GIT_FETCH_WITH_CLI=true
```

# How to run the sov-rollup-starter:
#### 1. Change the working directory:

```shell,test-ci
$ cd crates/rollup/
```

#### 2. If you want to run a fresh rollup, clean the database:

```sh,test-ci
$ make clean-db
```

#### 3. Start the rollup node:

```sh,test-ci
export SOV_PROVER_MODE=execute
```

This will compile and start the rollup node:

```shell,test-ci,bashtestmd:long-running,bashtestmd:wait-until=RPC
$ cargo run --bin node
```

#### 4. Submit a token creation transaction to the `bank` module:

```sh,test-ci
$ make test-create-token
```

#### 5. Note the transaction hash from the output of the above command

```text
Your batch was submitted to the sequencer for publication. Response: "Submitted 1 transactions"
0: 633764b4ac1e0a6259d786e4a2b8b916f16c2c9690359d8b53995fd6d80747cd
```

#### 6. Wait for the transaction to be submitted.
```sh,test-ci
$ make wait-ten-seconds
```

#### 7. To get the token address, fetch the events of the transaction hash from #5
```bash,test-ci
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getEventsByTxnHash","params":["633764b4ac1e0a6259d786e4a2b8b916f16c2c9690359d8b53995fd6d80747cd"],"id":1}' http://127.0.0.1:12345
{"jsonrpc":"2.0","result":[{"event_value":{"TokenCreated":{"token_address":"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72"}},"module_name":"bank","module_address":"sov1r5glamudyy9ysysfjkwu3wf9cjqs98e47tzc6pxuqlp48phqk36sthwg6h"}],"id":1}
```

#### 8. Test if token creation succeeded:

```sh,test-ci
$ make test-bank-supply-of
```

#### 9. The output of the above script:

```bash,test-ci,bashtestmd:compare-output
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"bank_supplyOf","params":{"token_address":"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72"},"id":1}' http://127.0.0.1:12345
{"jsonrpc":"2.0","result":{"amount":10000000},"id":1}
```

# How to run the sov-rollup-starter using celestia-da:
#### 1. Change the working directory:

```
$ cd crates/rollup/
```

#### 2. If you want to run a fresh rollup, clean the database:

```
$ make clean
```

#### 3. Start the Celestia local docker service:

```
$ make start
```

#### 4. Start the rollup node with the feature flag building with the celestia adapter:

This will compile and start the rollup node:

```
$ cargo run --bin rollup --no-default-features --features celestia_da
```

#### 5. Submit a token creation transaction to the `bank` module:

Using `CELESTIA=1` will enable the client to be built with Celestia support and submit the test token

```
$ CELESTIA=1 make test-create-token
```

#### 6. Note the transaction hash from the output of the above command

```text
Your batch was submitted to the sequencer for publication. Response: "Submitted 1 transactions"
0: 633764b4ac1e0a6259d786e4a2b8b916f16c2c9690359d8b53995fd6d80747cd
```


#### 7. To get the token address, fetch the events of the transaction hash from #5
```bash,test-ci
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"ledger_getEventsByTxnHash","params":["633764b4ac1e0a6259d786e4a2b8b916f16c2c9690359d8b53995fd6d80747cd"],"id":1}' http://127.0.0.1:12345
{"jsonrpc":"2.0","result":[{"event_value":{"TokenCreated":{"token_address":"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72"}},"module_name":"bank","module_address":"sov1r5glamudyy9ysysfjkwu3wf9cjqs98e47tzc6pxuqlp48phqk36sthwg6h"}],"id":1}
```

#### 8. Test if token creation succeeded:


```
$ make test-bank-supply-of
```

#### 9. The output of the above script:

```
$ curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"bank_supplyOf","params":{"token_address":"sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72"},"id":1}' http://127.0.0.1:12345
{"jsonrpc":"2.0","result":{"amount":10000000},"id":1}
```

## Enabling the prover
By default, demo-rollup disables proving (i.e. the default behavior is. If we want to enable proving, several options are available:

* `export SOV_PROVER_MODE=skip` Skips verification logic.
* `export SOV_PROVER_MODE=simulate` Run the rollup verification logic inside the current process.
* `export SOV_PROVER_MODE=execute` Run the rollup verifier in a zkVM executor.
* `export SOV_PROVER_MODE=prove` Run the rollup verifier and create a SNARK of execution.
