//! Integration test entry point.
//!
//! Cargo discovers tests in `tests/*.rs` but not in subdirectories.
//! This file includes the individual integration test modules.

mod integration {
    include!("integration/config_load_test.rs");
}

mod expand_integration {
    include!("integration/expand_test.rs");
}

mod bubblewrap_integration {
    include!("integration/bubblewrap_test.rs");
}
