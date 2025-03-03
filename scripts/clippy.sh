#!/bin/bash

# Add error handling and verbose output
set -e  # Exit on any error
set -x  # Print commands being executed

# Run clippy while ignoring tests and dependencies
cargo clippy \
    --no-deps \
    -p server \
    -p db-access \
    -p starknet-handler \
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