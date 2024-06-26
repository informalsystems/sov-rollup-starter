{
    description = "Nix development dependencies for ibc-rs";

    inputs = {
        nixpkgs.url = github:nixos/nixpkgs/nixpkgs-unstable;

        flake-utils.url = github:numtide/flake-utils;

        rust-overlay.url = github:oxalica/rust-overlay;

        sovereign-sdk-src = {
            flake = false;
            url = git+ssh://git@github.com/informalsystems/sovereign-sdk-wip?rev=fc1552b44acddc2674a783cd11f301a8b79cc4f3;
        };

        celestia-app-src = {
            flake = false;
            url = github:celestiaorg/celestia-app/v1.3.0;
        };

        celestia-node-src = {
            flake = false;
            url = github:celestiaorg/celestia-node/v0.12.0;
        };

        gaia-src = {
            flake = false;
            url = github:cosmos/gaia/v14.1.0;
        };
    };

    outputs = inputs:
        let
            utils = inputs.flake-utils.lib;
        in
            utils.eachSystem
            [
            "aarch64-darwin"
            "aarch64-linux"
            "x86_64-darwin"
            "x86_64-linux"
            ]
            (system: let
                nixpkgs = import inputs.nixpkgs {
                    inherit system;
                    overlays = [
                        inputs.rust-overlay.overlays.default
                    ];
                    config = {
                        permittedInsecurePackages = [
                            "openssl-1.1.1w"
                        ];
                    };
                };

                rust-bin = nixpkgs.rust-bin.stable.latest.complete;

                risc0-rust-tarball = builtins.fetchurl {
                    url = "https://github.com/risc0/rust/releases/download/v2024-04-22.0/rust-toolchain-x86_64-unknown-linux-gnu.tar.gz";
                    sha256 = "sha256:1jnjd5wv31ii1vknfdw2idbq1lcdgmzp1fp5ff8pbghid2hdp6ww";
                };

                risc0-circuit = builtins.fetchurl {
                    url = "https://risc0-artifacts.s3.us-west-2.amazonaws.com/zkr/ae5736a42189aec2f04936c3aee4b5441e48b26b4fa1fae28657cf50cdf3cae4.zip";
                    sha256 = "sha256:1r6ayg6m1ksphvigm8agdfr4h7j4npjaxhrn97qc5bl946j3cmxf";
                };

                risc0-rust = import ./nix/risc0.nix {
                    inherit nixpkgs rust-bin risc0-rust-tarball;
                };

                rollup-packages = import ./nix/rollup.nix {
                    inherit nixpkgs rust-bin risc0-rust risc0-circuit;
                    inherit (inputs) sovereign-sdk-src;
                };

                gaia = import ./nix/gaia.nix {
                    inherit nixpkgs;

                    inherit (inputs) gaia-src;
                };

                celestia-app = import ./nix/celestia-app.nix {
                    inherit nixpkgs;

                    inherit (inputs) celestia-app-src;
                };

                celestia-node = import ./nix/celestia-node.nix {
                    inherit nixpkgs;

                    inherit (inputs) celestia-node-src;
                };
            in {
                packages = {
                    inherit risc0-rust gaia celestia-app celestia-node;
                    inherit (rollup-packages) rollup rollup-guest-mock rollup-guest-celestia;

                    openssl = nixpkgs.openssl_1_1.dev;
                };
            });
}
