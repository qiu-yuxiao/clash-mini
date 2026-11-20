mod filter;
mod fingerprint;
mod verify_store;

#[cfg(test)]
mod tests;

pub use self::filter::{
    effective_filter_text, has_effective_filter_term, has_region_hint, is_dummy_node, is_incomplete_filter_input,
    is_valid_node_server, match_node_filter, pick_subset_fallback_node, promote_preferred_proxy_first,
};
pub use self::fingerprint::{
    RawProxyEntry, proxy_endpoint_fingerprint, resolve_proxy_clone_topology, set_runtime_node_fingerprints,
    set_runtime_tainted_banner_fps,
};
#[cfg(test)]
pub use self::verify_store::set_verify_record_for_test;
pub use self::verify_store::{
    HTTPS_VERIFY_MAX_FAILS, NodeVerifyRecord, clear_unverified_https_failures, confirmed_fake_node_names,
    has_pending_unverified_nodes, has_verify_failure_node, is_confirmed_fake_node, is_https_quarantined_fp,
    is_verified_true_node, record_https_verify_results,
};
