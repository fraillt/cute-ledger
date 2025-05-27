/// All logic related to client account balance management.
/// State is modified using events, which are created by handling commands
pub mod account;

/// Create account commands that later is executed by [`account`].
pub mod command;

/// Transaction processor interface, plus "in memory" implementation.
/// Coordinates all the logic from command parsing and processing
///
/// NOTE: Technically this interface is not necessary, but it might be
/// good integration point to replace in memory implementation with
/// something more sophisticated.
pub mod processor;

/// Ideally, this module should exists on its own crate, as a way to
/// bootstrap core logic. However, I want to use it for integration test
/// so I put it here.
pub mod bin_utils;
