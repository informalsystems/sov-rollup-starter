## Automation

### About
This setup is specifically to launch a rollup on AWS connected to the celestia mocha testnet. For a simpler setup which connects to an in-memory mock DA, please refer to [MOCK](MOCK_README.md)

This directory contains ansible playbooks to automate setting up the `sov-rollup-starter` binary on a remote AWS machine. The ansible playbooks can potentially work on any machine with two disks, but has been tested using the AWS machine mentioned below.

### Key generation
This is a one time step to generate the celestia keypair that will be used to post blobs. Follow the guide here [KEYGEN](./KEYGEN.md)

### Machine recommendations
* AWS [`c5ad.4xlarge`](https://aws.amazon.com/ec2/instance-types/c5/)
  * 16 cores
  * 2 x NVME SSD
  * 32 GB RAM
* Ubuntu 22.04 LTS.
* Open security group.
* Root volume >=100GB gp3, to accommodate build process of the rollup.

### Installation (Mac OS)
* Homebrew - https://brew.sh/
* Ansible
```
brew install ansible
ansible --version
ansible [core 2.16.5]
```
* go1.21.1 - install for your specific architecture from https://go.dev/dl/: [MacOS .pkg](https://go.dev/dl/go1.21.1.darwin-arm64.pkg)
```
▸ go version
go version go1.21.9 darwin/arm64
```

(!) It is important to have go 1.21 

### Ansible variables to edit
* [common](roles/common/defaults/main.yaml)
  * Primary variables to edit 
    * `aws_prometheus_remote_write_url`
    * `aws_prometheus_monitoring_label`
    * `aws_region`
* [data-availability](roles/data-availability/defaults/main.yaml)
  * Modify `cluster` to `testnet` or `mainnet` depending on the variables you want to pick
  * [testnet](roles/data-availability/defaults/testnet/variables.yaml)
  * [mainnet](roles/data-availability/defaults/mainnet/variables.yaml)
  * Primary variables to edit (described in [KEYGEN](./KEYGEN.md))
    * `key_name`
    * `key_address_filename`
    * `da_start_from`: This variable can to be edited for faster sync. Fetch latest height from any celestia RPC (eg: https://mocha.celenium.io/)
* [rollup](roles/rollup/defaults/main.yaml)
  *  Modify `cluster` to `testnet` or `mainnet` depending on the variables you want to pick
  * [testnet](roles/rollup/defaults/testnet/variables.yaml)
  * [mainnet](roles/rollup/defaults/mainnet/variables.yaml)
  * All the variables will likely need to be edited (variables are described in comments)
    * `rollup_da_start_height` can be set a few slots higher than `da_start_from`

### Steps to launch the rollup
* Launch the machine in AWS 
* Select `c5ad.4xlarge` as the instance type 
* Ensure public IP is attached
* Ensure a permissive security group for testnet 
* The only restriction, for instance, role is to ensure it can post data to AWS managed prometheus (out of the scope of this README)
* Ensure <aws_ssh_key>.pem is part of the ssh agent: `ssh-add ~/.ssh/YourAWSKey.pem`. This key is needed to ansible provision machine 
* Ensure that your GitHub ssh keys is a part of of the ssh agent, so it can fetch code from private WIP repos. This key is needed to get access to GitHub repo. If repo is publicly accessible it is not needed

```bash
ssh-add -l
2048 SHA256:udAui6vtUjoAtuza7l+x5tZsoq+cAzvD5TNjh6SuhyA ~/.ssh/YourAWSKey.pem (RSA)
2048 SHA256:Bxv9vtL64zz2QuhEysRiF2s5WPLVp99YpgdNfqJe5u4 ~/.ssh/github_id_rsa (RSA)
```

* Run the ansible command to set up the machine from the automation folder

```bash
cd automation
```

```bash
$ ansible-playbook setup.yaml -i '<ip_address>,' -u ubuntu --private-key ~/.ssh/<aws_ssh_key>.pem -e 'ansible_ssh_common_args="-o ForwardAgent=yes" -e 'switches=cdr' -e data_availability_role=celestia'
PLAY [Playbook Runner] ********************************************************************************************************************************************************************
...

PLAY RECAP ********************************************************************************************************************************************************************************
<ip_address>             : ok=93   changed=30   unreachable=0    failed=0    skipped=36   rescued=0    ignored=1
```

This is expected output. Please note that `failed` should be `0`.

### Notes
* `da_start_from` and `rollup_da_start_height` make this significantly faster by starting from a trusted hash. check: [da_rpc_queries.py](scripts/python/da_rpc_queries.py)
* The next points are only relevant if not using `da_start_from` 
* The DA layer catch up takes some time to catch up if syncing from genesis, so if the above command gets stuck during the task named `Loop until height is greater than to_height`, it can be ctrl+c'd and re-run.
* Progress can also be monitored by ssh-ing to the machine and running the following command after switching to the `sovereign` user
```
$ sudo su - sovereign
$ celestia header sync-state --node.store /mnt/da
{
  "result": {
    "id": 2,
    "height": 1430909,
    "from_height": 1387516,
    "to_height": 1387831,
    "from_hash": "EC63CCC2D4F6E36FB42B4C1BF302D21A428CB45617B4CD4FF0AE82A4BE85B6F1",
    "to_hash": "C6048F0C08D4FAE92CAAF9569BF483594127C2D3D79F49FE45EA3005C7FAC5AF",
    "start": "2024-03-15T12:38:54.119717467Z",
    "end": "2024-03-15T12:38:54.11985308Z"
  }
```
* Once the DA light client catches up `height` will be greater than `to_height`

### Structure
The automation folder consists of 3 ansible roles which are executed on a remote machine
* `common`
  * Installs base ubuntu dependencies
  * Rust, Golang, compiler tools
  * Disk setup and mounting
  * User creation (sovereign user)
  * Kernel and OS tuning
  * Prometheus installation
  * Chrony for time sync
* `data-availability`
  * Download and install celestia
  * Upload user generated keys from local to the remote machine
  * Start the DA service
  * Wait for the DA service to be caught up
* `rollup`
  * Download the `sov-rollup-starter` repository
  * Checkout the specific commit mentioned in the variables
  * Edit configuration files based on variables
  * Start the rollup binary as a `systemd` service

### Usage
The ansible playbook behavior can be changed by modifying the `switches` variable
* switches
  * `c`: common
  * `d`: data availability
  * `r`: rollup
* `data_availability_role` controls if `mock` or `celestia` DA is used
  * `-e 'data_availability_role=celestia'`
* Setting up the machine from scratch: `-e 'switches=cdr'`
  * All the above installations are completed
  * rollup service is started
* Updating the rollup binary: `-e 'switches=r'`
  * rollup service is stopped
  * git is updated
  * rollup binary is rebuilt
  * rollup service is started
* Updating the rollup binary and wiping the rollup's data storage directory `-e 'switches=r' -e wipe=true`


### Troubleshooting

Status of the service:

```bash
sudo systemctl status rollup
```

Service logs:

```bash
journalctl -u rollup
```

Non panic log messages are also available in the file:
```bash
tail -f /mnt/logs/rollup.log.<DATE>
```