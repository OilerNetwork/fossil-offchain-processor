.DEFAULT_GOAL := help

##@ Build

.PHONY: build
build: ## Build the project in release mode.
	cargo build --release

.PHONY: build-debug
build-debug: ## Build the project in debug mode.
	cargo build

##@ Test

.PHONY: test
test: ## Run all tests.
	cargo test --workspace --all-features

##@ Linting

.PHONY: fmt
fmt: ## Format code with rustfmt.
	cargo +nightly fmt

.PHONY: clippy
clippy: ## Run clippy linter with project-specific settings.
	cargo +nightly clippy \
		--no-deps \
		-- \
		-W clippy::branches_sharing_code \
		-W clippy::clear_with_drain \
		-W clippy::derive_partial_eq_without_eq \
		-W clippy::empty_line_after_outer_attr \
		-W clippy::equatable_if_let \
		-W clippy::imprecise_flops \
		-W clippy::iter_on_empty_collections \
		-W clippy::iter_with_drain \
		-W clippy::large_stack_frames \
		-W clippy::manual_clamp \
		-W clippy::mutex_integer \
		-W clippy::needless_pass_by_ref_mut \
		-W clippy::nonstandard_macro_braces \
		-W clippy::or_fun_call \
		-W clippy::path_buf_push_overwrite \
		-W clippy::read_zero_byte_vec \
		-W clippy::redundant_clone \
		-W clippy::suboptimal_flops \
		-W clippy::suspicious_operation_groupings \
		-W clippy::trailing_empty_array \
		-W clippy::trait_duplication_in_bounds \
		-W clippy::transmute_undefined_repr \
		-W clippy::trivial_regex \
		-W clippy::tuple_array_conversions \
		-W clippy::uninhabited_references \
		-W clippy::unused_peekable \
		-W clippy::unused_rounding \
		-W clippy::useless_let_if_seq \
		-W clippy::use_self \
		-W clippy::missing_const_for_fn \
		-W clippy::empty_line_after_doc_comments \
		-W clippy::iter_on_single_items \
		-W clippy::match_same_arms \
		-W clippy::doc_markdown \
		-W clippy::unnecessary_struct_initialization \
		-W clippy::string_lit_as_bytes \
		-W clippy::explicit_into_iter_loop \
		-W clippy::explicit_iter_loop \
		-W clippy::manual_string_new \
		-W clippy::naive_bytecount \
		-W clippy::needless_bitwise_bool \
		-W clippy::zero_sized_map_values \
		-W clippy::single_char_pattern \
		-W clippy::needless_continue \
		-W clippy::single_match \
		-W clippy::single_match_else \
		-W clippy::needless_match \
		-W clippy::needless_late_init \
		-W clippy::redundant_pattern_matching \
		-W clippy::redundant_pattern \
		-W clippy::redundant_guards \
		-W clippy::collapsible_match \
		-W clippy::match_single_binding \
		-W clippy::match_ref_pats \
		-W clippy::match_bool \
		-D clippy::needless_bool \
		-W clippy::unwrap_used \
		-W clippy::expect_used

.PHONY: lint-codespell
lint-codespell: ensure-codespell ## Check for spelling mistakes.
	codespell

.PHONY: ensure-codespell
ensure-codespell:
	@if ! command -v codespell &> /dev/null; then \
		echo "codespell not found. Please install it with 'pip install codespell'" >&2; \
		exit 1; \
	fi

.PHONY: lint
lint: fmt clippy lint-codespell ## Run all linters.

##@ Pull Request

.PHONY: pr
pr: ## Prepare code for a pull request.
	make lint && \
	make test

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: setup setup-rust setup-foundry setup-risc0 setup-starknet setup-platform init-repo

setup: setup-rust setup-foundry setup-risc0 setup-starknet setup-platform init-repo
	@echo "✅ All dependencies installed successfully!"

setup-rust:
	@echo "🔧 Checking Rust installation..."
	@if ! command -v rustup &> /dev/null; then \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	else \
		echo "✅ Rust already installed"; \
	fi
	@if ! rustup toolchain list | grep -q "nightly"; then \
		echo "Installing Rust nightly..."; \
		rustup toolchain install nightly; \
		rustup default nightly; \
	else \
		echo "✅ Rust nightly already installed"; \
	fi