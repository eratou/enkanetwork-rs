#[cfg(all(not(target_arch = "wasm32"),feature="async-io"))]
pub mod io_async;
#[cfg(any(target_arch = "wasm32",not(feature="async-io")))]
pub mod io_std;
#[cfg(all(not(target_arch = "wasm32"),feature="async-io"))]
pub use io_async::*;
#[cfg(any(target_arch = "wasm32",not(feature="async-io")))]
pub use io_std::*;

mod memory_cache;
pub use memory_cache::MemoryCache;
