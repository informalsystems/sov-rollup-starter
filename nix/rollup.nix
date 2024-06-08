{
    nixpkgs
,   rust-bin
,   risc0-rust
,   sovereign-sdk-src
,   risc0-circuit
}:
let
    sov-ibc-hash = "sha256-aZXC6dYQJlPpx9pkDexGe41roZaKiduJ4kmcY4PJs3w=";
    ibc-rs-hash = "sha256-n1JjjHZlkmyxQ/LPEF1hCCKnz8Yrje3erXsMW9uGqL0=";

    rollup-guest-src = nixpkgs.stdenv.mkDerivation {
        name = "rollup-guest-src";
        src = ../crates;
        dontBuild = true;

        installPhase = ''
            mkdir -p $out/crates $out/vendor
            cp -r . $out/crates
            cp -r ${sovereign-sdk-src} $out/vendor/sovereign-sdk
            cp ${../Cargo.toml} $out/Cargo.toml
            cp ${../constants.json} $out/constants.json
        '';
    };


    rollup-src = nixpkgs.stdenv.mkDerivation {
        name = "rollup-src";
        src = ../crates;
        dontBuild = true;

        installPhase = ''
            mkdir -p $out/crates $out/vendor
            cp -r . $out/crates
            cp -r ${sovereign-sdk-src} $out/vendor/sovereign-sdk
            cp ${../Cargo.toml} $out/Cargo.toml
            cp ${../Cargo.lock} $out/Cargo.lock
            cp ${../constants.json} $out/constants.json
        '';
    };

    rollup-guest-mock = nixpkgs.rustPlatform.buildRustPackage {
        name = "rollup-guest-mock";

        src = rollup-guest-src;

        sourceRoot = "rollup-guest-src/crates/provers/risc0/guest-mock";

        cargoLock = {
            lockFile = ../crates/provers/risc0/guest-mock/Cargo.lock;
            outputHashes = {
                "jmt-0.9.0" = "sha256-pq1v6FXS//6Dh+fdysQIVp+RVLHdXrW5aDx3263O1rs=";
                "crypto-bigint-0.5.2" = "sha256-9rh8z3vwOQ7/mtzVbyADoRWgTzARF/nkhBwfKb7+A6I=";
                "curve25519-dalek-4.1.0" = "sha256-H8YMea3AIcUn9NGRfataNjCTzCK4NAjo4ZhWuPfT6ts=";
                "sha2-0.10.8" = "sha256-vuFQFlbDXEW+n9+Nx2VeWanggCSd6NZ+GVEDFS9qZ2M=";
                "risc0-cycle-utils-0.3.0" = "sha256-5dA62v1eqfyZBny4s3YlC2Tty7Yfd/OAVGfTlLBgypk=";
                "sov-celestia-client-0.1.0" = sov-ibc-hash;
                "ibc-app-transfer-0.53.0" = ibc-rs-hash;
            };
        };

        nativeBuildInputs = [
            rust-bin
            nixpkgs.lld
        ];

        doCheck = false;

        CONSTANTS_MANIFEST = rollup-guest-src;

        buildPhase = ''
            RUSTC=${risc0-rust}/bin/rustc \
                CARGO_ENCODED_RUSTFLAGS=$'-C\x1fpasses=loweratomic\x1f-C\x1flink-arg=-Ttext=0x00200800\x1f-C\x1flink-arg=--fatal-warnings\x1f-C\x1fpanic=abort\x1f-C\x1flinker=lld' \
                cargo build --release --target riscv32im-risc0-zkvm-elf -p guest-mock-starter
        '';

        installPhase = ''
            mkdir -p $out
            cp target/riscv32im-risc0-zkvm-elf/release/mock_da $out/
        '';
    };

    rollup-guest-celestia = nixpkgs.rustPlatform.buildRustPackage {
        name = "rollup-guest-celestia";

        src = rollup-guest-src;

        sourceRoot = "rollup-guest-src/crates/provers/risc0/guest-celestia";

        cargoLock = {
            lockFile = ../crates/provers/risc0/guest-celestia/Cargo.lock;
            outputHashes = {
                "celestia-proto-0.1.0" = "sha256-iUgrctxdJUyhfrEQ0zoVj5AKIqgj/jQVNli5/K2nxK0=";
                "crypto-bigint-0.5.2" = "sha256-9rh8z3vwOQ7/mtzVbyADoRWgTzARF/nkhBwfKb7+A6I=";
                "curve25519-dalek-4.1.0" = "sha256-H8YMea3AIcUn9NGRfataNjCTzCK4NAjo4ZhWuPfT6ts=";
                "jmt-0.9.0" = "sha256-pq1v6FXS//6Dh+fdysQIVp+RVLHdXrW5aDx3263O1rs=";
                "nmt-rs-0.1.0" = "sha256-jcHbqyIKk8ZDDjSz+ot5YDxROOnrpM4TRmNFVfNniwU=";
                "sha2-0.10.8" = "sha256-vuFQFlbDXEW+n9+Nx2VeWanggCSd6NZ+GVEDFS9qZ2M=";
                "tendermint-0.32.0" = "sha256-FtY7a+hBvQryATrs3mykCWFRe8ABTT6cuf5oh9IBElQ=";
                "risc0-cycle-utils-0.3.0" = "sha256-5dA62v1eqfyZBny4s3YlC2Tty7Yfd/OAVGfTlLBgypk=";
                "sov-celestia-client-0.1.0" = sov-ibc-hash;
                "ibc-app-transfer-0.53.0" = ibc-rs-hash;
            };
        };

        nativeBuildInputs = [
            rust-bin
            nixpkgs.lld
            nixpkgs.protobuf
        ];

        doCheck = false;

        CONSTANTS_MANIFEST = rollup-guest-src;

        buildPhase = ''
            RUSTC=${risc0-rust}/bin/rustc \
                CARGO_ENCODED_RUSTFLAGS=$'-C\x1fpasses=loweratomic\x1f-C\x1flink-arg=-Ttext=0x00200800\x1f-C\x1flink-arg=--fatal-warnings\x1f-C\x1fpanic=abort\x1f-C\x1flinker=lld' \
                cargo build --release --target riscv32im-risc0-zkvm-elf -p sov-demo-prover-guest-celestia
        '';

        installPhase = ''
            mkdir -p $out
            cp target/riscv32im-risc0-zkvm-elf/release/rollup $out/
        '';
    };


    rollup = nixpkgs.rustPlatform.buildRustPackage {
        name = "sov-rollup-starter";

        src = rollup-src;

        cargoLock = {
            lockFile = ../Cargo.lock;
            outputHashes = {
                "celestia-proto-0.1.0" = "sha256-iUgrctxdJUyhfrEQ0zoVj5AKIqgj/jQVNli5/K2nxK0=";
                "jmt-0.9.0" = "sha256-pq1v6FXS//6Dh+fdysQIVp+RVLHdXrW5aDx3263O1rs=";
                "nmt-rs-0.1.0" = "sha256-jcHbqyIKk8ZDDjSz+ot5YDxROOnrpM4TRmNFVfNniwU=";
                "tendermint-0.32.0" = "sha256-FtY7a+hBvQryATrs3mykCWFRe8ABTT6cuf5oh9IBElQ=";
                "crypto-bigint-0.5.2" = "sha256-9rh8z3vwOQ7/mtzVbyADoRWgTzARF/nkhBwfKb7+A6I=";
                "curve25519-dalek-4.1.0" = "sha256-H8YMea3AIcUn9NGRfataNjCTzCK4NAjo4ZhWuPfT6ts=";
                "risc0-cycle-utils-0.3.0" = "sha256-5dA62v1eqfyZBny4s3YlC2Tty7Yfd/OAVGfTlLBgypk=";
                "rockbound-1.0.0" = "sha256-aDrNegRfsSwiNw4XLsE4rpUYDZn2N59UJbGZ6mpY180=";
                "sov-celestia-client-0.1.0" = sov-ibc-hash;
                "ibc-0.53.0" = ibc-rs-hash;
            };
        };

        doCheck = false;
        # buildType = "debug";
        buildNoDefaultFeatures = true;
        buildFeatures = [ "celestia_da" ];

        PKG_CONFIG_PATH = "${nixpkgs.openssl.dev}/lib/pkgconfig";

        ROLLUP_ELF_PATH = "${rollup-guest-celestia}/rollup";
        RECURSION_SRC_PATH = "${risc0-circuit}";
        CONSTANTS_MANIFEST = rollup-src;

        nativeBuildInputs = [
            nixpkgs.pkg-config
            nixpkgs.protobuf
            nixpkgs.rustPlatform.bindgenHook
        ];

    };
in
{
    inherit rollup rollup-guest-celestia rollup-guest-mock;
}