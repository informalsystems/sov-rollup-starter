

lint:
	SKIP_GUEST_BUILD=1 cargo fmt --all -- --check
	SKIP_GUEST_BUILD=1 cargo check
	SKIP_GUEST_BUILD=1 cargo check --features celestia_da --no-default-features
	SKIP_GUEST_BUILD=1 cargo clippy
	SKIP_GUEST_BUILD=1 cargo clippy --features celestia_da --no-default-features
