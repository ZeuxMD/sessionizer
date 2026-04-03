PNPM := pnpm
CARGO := cargo

.PHONY: install format format-check lint typecheck test build check

install:
	corepack enable
	$(PNPM) install --no-frozen-lockfile

format:
	$(PNPM) format:write
	$(CARGO) fmt --manifest-path src-tauri/Cargo.toml --all

format-check:
	$(PNPM) format
	$(CARGO) fmt --manifest-path src-tauri/Cargo.toml --all --check

lint:
	$(PNPM) lint
	$(CARGO) clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

typecheck:
	$(PNPM) typecheck
	$(CARGO) check --manifest-path src-tauri/Cargo.toml --all-targets

test:
	$(PNPM) test
	$(CARGO) test --manifest-path src-tauri/Cargo.toml --all-targets

build:
	$(PNPM) build
	$(CARGO) build --manifest-path src-tauri/Cargo.toml

check: format-check lint typecheck test build
