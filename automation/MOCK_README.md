## Automation

### About
This README is for launching a rollup on AWS that uses a mock da layer (internal to the rollup machine's memory). For a more complete testnet setup connected to celestia, please refer to [README](README.md)
This directory contains ansible playbooks to automate setting up the `sov-rollup-starter` binary on a remote AWS machine. The ansible playbooks can potentially work on any machine with two disks, but has been tested using the AWS machine mentioned below.

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
ansible-playbook setup.yaml -i '<ip_address>,' -u ubuntu --private-key ~/.ssh/<aws_ssh_key>.pem -e 'ansible_ssh_common_args="-o ForwardAgent=yes" -e 'switches=cdr' -e data_availability_role=mock'
```

### Usage
The ansible playbook behavior can be changed by modifying the `switches` variable
* switches
    * `c`: common
    * `d`: data availability
    * `r`: rollup
* `data_availability_role` controls if `mock` or `celestia` DA is used
    * `-e 'data_availability_role=mock'`
* Setting up the machine from scratch: `-e 'switches=cdr'`
    * All the above installations are completed
    * rollup service is started
* Updating the rollup binary: `-e 'switches=r'`
    * rollup service is stopped
    * git is updated
    * rollup binary is rebuilt
    * rollup service is started
* Updating the rollup binary and wiping the rollup's data storage directory `-e 'switches=r' -e wipe=true`
