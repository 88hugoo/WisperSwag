//! Procesamiento del texto transcrito.
//!
//! Todas las transformaciones locales son deterministas, se ejecutan sin conexión y
//! pueden activarse o desactivarse individualmente. El orden completo está en
//! [`pipeline`].

pub mod cleanup;
pub mod commands;
pub mod format;
pub mod hallucination;
pub mod modes;
pub mod normalize;
pub mod pipeline;
pub mod snippets;
pub mod spoken;
pub mod vocab;
