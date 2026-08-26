//! Test-only `ChunkStorageBackend` fakes (M2-B08's own self-test fixtures) — proving
//! `chunk_soak::run_soak`'s own analysis logic is correct before it is ever trusted
//! against a real `AnvilDiskBackend` (Context, "the harness's own comparison/analysis
//! functions correct against deliberately-broken fakes").

pub mod corrupting_backend;
