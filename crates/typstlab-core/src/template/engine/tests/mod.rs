//! Tests for template engine
//!
//! This module contains comprehensive tests for the template engine,
//! organized into focused submodules for better maintainability.

use super::*;

// Test helper functions
mod helpers;

// TokenStream tests
mod tokenstream;
mod tokenstream_performance;

// Rendering tests
mod render_basic;
mod render_escaping;
mod render_loops;

// Error and edge case tests
mod errors;
mod timeouts;
