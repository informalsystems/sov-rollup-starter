### Celestia DA key generation
* This is a one time process to generate a celestia keypair for the sequencer to post blobs to the celestia DA layer
* Ensure go (version 1.21.1) is installed locally
* Ensure dependencies are installed for celestia - https://docs.celestia.org/nodes/environment#install-dependencies
* Checkout `celestia-node`
```
git clone https://github.com/celestiaorg/celestia-node.git
```
* Checkout the correct version `tags/v0.12.4`
```
git checkout tags/v0.12.4
```
* Build the celestia keygen tool
```
make cel-key
```
* Create the key that the sequencer would be using to post blobs to the DA layer (we're using `mocha` as the p2p network because we're generating keys for the mocha testnet)
```
./cel-key add <key_name> --node.type light --p2p.network mocha
```
* Save the seed phrase
* Save the celestia address
* Update [testnet](roles/data-availability/defaults/testnet/variables.yaml) with the key information:
  * `key_name` should be the same as the one you create the key with.
  * `key_address_filename` should be the corresponding `.address` file name.
  * Both the above should be visible in `ls -lahtr ~/.celestia-light-mocha-4/keys/keyring-test`
* Update [testnet](roles/rollup/defaults/testnet/variables.yaml) with the celestia address:
  * `sequencer_self_da_address`: This should be the address used to post blobs to celestia
  * `sequencer_genesis_da_address`: This should be the genesis sequencer
  * In most cases, the above two variables will have the same value of the celestia address generated in the previous steps
  * Ensure that the address for the key you generated has funds to post data to the testnet
* Create `.keys` folder at the root of the repository
```
cd sov-rollup-starter-wip
mkdir -p .keys
mkdir -p .keys/<key_name>
cp ~/.celestia-light-mocha-4/keys/keyring-test/<key_name>.info ../sov-rollup-starter-wip/.keys/<key_name>/
cp ~/.celestia-light-mocha-4/keys/keyring-test/<key_address>.address ../sov-rollup-starter-wip/.keys/<key_name>/
```
* If the files are moved to the above location, nothing else needs to be changed in testnet variables file
* At the end of this process both the `.info` and `.address` files should be present inside following folder `sov-rollup-starter-wip/.keys/<keyname>`
