.PHONY: bootstrap dev down logs test quality verify-assets asset-index full-asset-catalog protocol e2e

bootstrap:
	cp .env.example .env
	pnpm install
	pnpm protocol:generate
	pnpm assets:index
	pnpm assets:catalog:full

dev:
	docker compose up --build

down:
	docker compose down --remove-orphans

logs:
	docker compose logs -f --tail=200

test:
	pnpm test:web
	cargo test --manifest-path apps/server/Cargo.toml

e2e:
	pnpm test:e2e

quality:
	pnpm protocol:generate
	cargo fmt --manifest-path apps/server/Cargo.toml -- --check
	cargo test --manifest-path apps/server/Cargo.toml
	cargo clippy --manifest-path apps/server/Cargo.toml --all-targets --all-features -- -D warnings
	pnpm test:web
	pnpm build:web
	pnpm architecture:validate
	npm audit --prefix apps/web
	pnpm assets:verify
	pnpm assets:validate:slice1
	pnpm assets:validate:original-flow
	pnpm assets:validate:visible-world
	pnpm assets:validate:localization
	pnpm assets:validate:full
	pnpm building:validate
	pnpm test:assets
	pnpm scene:test
	pnpm scene:validate

verify-assets:
	pnpm assets:verify

asset-index:
	pnpm assets:index

full-asset-catalog:
	pnpm assets:catalog:full
	pnpm assets:validate:full

protocol:
	pnpm protocol:generate
