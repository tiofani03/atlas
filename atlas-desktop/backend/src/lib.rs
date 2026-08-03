//! Atlas Desktop backend library target.
//!
//! The backend ships as a binary (`main.rs`) but exposes its router and state
//! through this lib target so integration tests can drive the full HTTP API.

pub mod handlers;
pub mod routes;
pub mod state;

pub use routes::create_router as create_test_router;
pub use state::AppState;
