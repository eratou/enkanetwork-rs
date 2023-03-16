#[cfg(all(not(target_arch = "wasm32"),feature="async-io"))]
pub mod io_async;
#[cfg(any(target_arch = "wasm32",not(feature="async-io")))]
pub mod io_std;
#[cfg(all(not(target_arch = "wasm32"),feature="async-io"))]
use io_async::*;
#[cfg(any(target_arch = "wasm32",not(feature="async-io")))]
use io_std::*;

#[cfg(feature="redis")]
mod redis;
#[cfg(feature="redis")]
pub use self::redis::MemoryCache;
#[cfg(not(feature="redis"))]
mod memory_cache;
#[cfg(not(feature="redis"))]
pub use memory_cache::MemoryCache;
