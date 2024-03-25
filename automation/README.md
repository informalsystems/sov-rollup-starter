## Automation

### About
This directory contains ansible playbooks to automate setting up the `sov-rollup-starter` binary on a remote AWS machine. The ansible playbooks can potentially work on any machine with two disks, but has been tested using the AWS machine mentioned below.

### Key generation
This is a one time step to generate the celestia keypair that will be used to post blobs. Follow the guide here [KEYGEN](./KEYGEN.md)

### Machine recommendations
* AWS `c5ad.4xlarge`
  * 16 cores
  * 2 x NVME SSD
  * 32 GB RAM
* Ubuntu 22.04
* Open security group

### Installation (Mac OS)
* Homebrew - https://brew.sh/
* Ansible
```
brew install ansible
```
* go1.22.0 - install for your specific architecture from https://go.dev/dl/
```
https://go.dev/dl/go1.22.0.darwin-arm64.pkg
```

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
* [rollup](roles/rollup/defaults/main.yaml)
  *  Modify `cluster` to `testnet` or `mainnet` depending on the variables you want to pick
  * [testnet](roles/rollup/defaults/testnet/variables.yaml)
  * [mainnet](roles/rollup/defaults/mainnet/variables.yaml)
  * All the variables will likely need to be edited (variables are described in comments)

### Steps to launch the rollup
* Launch the machine in AWS 
* Select `c5ad.4xlarge` as the instance type 
* Ensure public IP is attached 
* Ensure a permissive security group for testnet 
* The only restriction for instance role is to ensure it can post data to AWS managed prometheus (out of the scope of this README)
* Ensure <aws_ssh_key>.pem is part of the ssh agent. Verify with
```
ssh-add -l
```
* Run the ansible command to set up the machine from the automation folder
```
ansible-playbook setup.yaml -i '<ip_address>,' -u ubuntu --private-key ~/.ssh/<aws_ssh_key>.pem -e 'ansible_ssh_common_args="-o ForwardAgent=yes" -e 'switches=cdr' '
```

### Notes
* The DA layer catch up takes some time currently, so if the above command gets stuck during the task named `Loop until height is greater than to_height`, it can be ctrl+c'd and re-run.
* The script will block there again while the DA light client is catching up (TBD: check if snapshots are feasible)
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
The ansible playbook can be used in two ways
* Setting up the machine from scratch
  * All the above installations are completed
  * rollup service is started
* Updating the rollup binary
  * rollup service is stopped
  * git is updated
  * rollup binary is rebuilt
  * rollup service is started
  * OPTIONALLY - wipe the rollup's data storage directory