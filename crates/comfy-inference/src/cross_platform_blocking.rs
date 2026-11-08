// Standalone (non-Windows) implementation of
// [`crate::cross_platform_blocking::run_blocking`].
//
// Mirrors the Windows variant (`windows_backend.rs`) but is compiled on every
// platform so the HTTP-backed backends that call `run_blocking`
// (`flash_attn_backend`, and `windows_backend` when `comfyui-bridge` is
// enabled on Unix) share one helper without cfg-gating each call site.
//
// This file is included via `include!` from the `#[cfg(not(any(windows,
// feature = "comfyui-bridge")))]`-gated `cross_platform_blocking` module in
// `lib.rs`; the Windows half of that module re-exports
// `windows_backend::run_blocking` instead.
//
// Note: kept as `//` comments because the file is included via `include!` from
// a module body, where inner `//!` doc comments before items are not permitted.

use tokio::runtime::Handle;

/// Run a blocking closure on the current thread. When called from inside a
/// tokio multi-threaded runtime, this uses `block_in_place` to transition the
/// worker thread to a blocking thread (so the reqwest blocking client's
/// internal runtime can be created/dropped without panicking with
/// "Cannot drop a runtime in a context where blocking is not allowed").
/// When called outside a tokio runtime (e.g. tests), the closure runs
/// directly.
pub fn run_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    match Handle::try_current() {
        Ok(_) => tokio::task::block_in_place(f),
        Err(_) => f(),
    }
}
