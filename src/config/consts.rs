/// Default fuel level for WASM execution (100 million instructions)
pub const DEFAULT_FUEL_LEVEL: u64 = 100_000_000;
/// Minimum allowed fuel level (1 million instructions)
pub const MIN_FUEL_LEVEL: u64 = 1_000_000;
/// Maximum allowed fuel level (500 million instructions) - security limit
pub const MAX_FUEL_LEVEL: u64 = 500_000_000;
