//! Sources.
//!
//! # General operations for all sources
//!
//! ## Refreshing local index
//!
//! Refreshing the index of a source and keeping a cached version of it locally.
//!
//! ## Searching fonts
//!
//! It should be possible to search through the indexed fonts quickly.
//!
//! # Advanced operations
//!
//! ## Generating new indices
//!
//! Some sources require generating an index (e.g. Google Fonts). FontPM should
//! support doing so within the CLI itself, instead of relying on outside
//! programs to.

mod google_fonts;
