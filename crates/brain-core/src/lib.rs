//! # Brain Core
//!
//! Primitivas, constantes y tipos compartidos de base para Local Brain.

/// Versión actual del paquete core.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_is_not_empty() {
        assert!(!CORE_VERSION.is_empty());
    }
}
