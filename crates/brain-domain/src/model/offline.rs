//! Política y validación de Modo Offline Estricto (SRS §29, §51).
//!
//! Garantiza que Local Brain funcione 100% de manera local en el dispositivo del usuario,
//! impidiendo la salida inadvertida de datos hacia la red pública o APIs en la nube.

use serde::{Deserialize, Serialize};

use crate::model::DomainError;

/// Política de aislamiento de red y operación offline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflinePolicy {
    /// Indica si el modo offline estricto se encuentra activado.
    pub enabled: bool,
    /// Indica si se permiten conexiones a interfaces de bucle invertido local (localhost, 127.0.0.1, ::1).
    pub allow_localhost: bool,
}

impl Default for OfflinePolicy {
    fn default() -> Self {
        Self::strict()
    }
}

impl OfflinePolicy {
    /// Modo offline estricto: bloquea cualquier host remoto, permitiendo exclusivamente bucle invertido local.
    pub const fn strict() -> Self {
        Self {
            enabled: true,
            allow_localhost: true,
        }
    }

    /// Modo permisivo: no bloquea llamadas de red salientes.
    pub const fn permissive() -> Self {
        Self {
            enabled: false,
            allow_localhost: true,
        }
    }

    /// Modo air-gapped total: no permite ni siquiera localhost (para pruebas de aislamiento absoluto).
    pub const fn air_gapped() -> Self {
        Self {
            enabled: true,
            allow_localhost: false,
        }
    }

    /// Valida si una URL dada cumple con las restricciones de la política offline.
    ///
    /// URLs como `http://localhost:8081/embedding`, `http://127.0.0.1:8081`, o
    /// `postgres://localbrain:secret@localhost:5433/db` son aceptadas si `allow_localhost` es true.
    /// Cualquier URL dirigida a IPs públicas, redes WAN o dominios remotos (e.g. `api.openai.com`)
    /// es rechazada de inmediato retornando `DomainError::OfflineViolation`.
    pub fn validate_url(&self, url_str: &str) -> Result<(), DomainError> {
        if !self.enabled {
            return Ok(());
        }

        let host = extract_host_from_url(url_str).ok_or_else(|| {
            DomainError::OfflineViolation(format!(
                "URL inválida o sin host identificable en modo offline: '{url_str}'"
            ))
        })?;

        if is_loopback_or_local_host(&host) {
            if self.allow_localhost {
                Ok(())
            } else {
                Err(DomainError::OfflineViolation(format!(
                    "Acceso a bucle invertido local '{host}' denegado por política air-gapped total"
                )))
            }
        } else {
            Err(DomainError::OfflineViolation(format!(
                "Acceso a red externa o remota denegado en modo offline estricto: '{host}' (URL: '{url_str}')"
            )))
        }
    }
}

/// Extrae el host (nombre de dominio o dirección IP) a partir de una URL textual.
fn extract_host_from_url(url_str: &str) -> Option<String> {
    let trimmed = url_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Si es socket UNIX o ruta de archivo local
    if trimmed.starts_with("unix://") || trimmed.starts_with('/') {
        return Some("localhost".to_string());
    }

    // Remover esquema ("http://", "postgres://", etc.)
    let after_scheme = if let Some(idx) = trimmed.find("://") {
        &trimmed[idx + 3..]
    } else {
        trimmed
    };

    // Remover información de usuario ("user:password@host")
    let after_auth = if let Some(idx) = after_scheme.rfind('@') {
        &after_scheme[idx + 1..]
    } else {
        after_scheme
    };

    // Remover path y query ("host:port/path?query")
    let authority = after_auth
        .split('/')
        .next()?
        .split('?')
        .next()?
        .split('#')
        .next()?;

    // Si es IPv6 entre corchetes "[::1]:8080"
    if authority.starts_with('[') {
        if let Some(end_bracket) = authority.find(']') {
            return Some(authority[1..end_bracket].to_string());
        }
    }

    // Remover puerto ":8080"
    let host = authority.split(':').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// Determina si un host corresponde a una interfaz de bucle invertido (*loopback*) o local.
fn is_loopback_or_local_host(host: &str) -> bool {
    let lower = host.to_lowercase();
    if lower == "localhost" || lower == "127.0.0.1" || lower == "::1" || lower == "0.0.0.0" {
        return true;
    }

    // Prefijos IPv4 loopback (127.0.0.0/8)
    if lower.starts_with("127.") {
        let parts: Vec<&str> = lower.split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
            return true;
        }
    }

    // Prefijos de nombres locales .local o .internal
    if lower.ends_with(".local") || lower.ends_with(".localhost") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissive_policy_allows_all() {
        let policy = OfflinePolicy::permissive();
        assert!(policy
            .validate_url("https://api.openai.com/v1/chat")
            .is_ok());
        assert!(policy.validate_url("http://192.168.1.50:8000").is_ok());
        assert!(policy
            .validate_url("http://127.0.0.1:8081/embedding")
            .is_ok());
    }

    #[test]
    fn test_strict_policy_allows_localhost_and_loopback() {
        let policy = OfflinePolicy::strict();
        assert!(policy
            .validate_url("http://localhost:8081/embedding")
            .is_ok());
        assert!(policy
            .validate_url("http://127.0.0.1:8081/embedding")
            .is_ok());
        assert!(policy.validate_url("http://127.0.0.2:9000").is_ok());
        assert!(policy.validate_url("http://[::1]:8081/embedding").is_ok());
        assert!(policy
            .validate_url("postgres://localbrain:secret@localhost:5433/local_brain")
            .is_ok());
        assert!(policy.validate_url("unix:///var/run/docker.sock").is_ok());
    }

    #[test]
    fn test_strict_policy_rejects_external_hosts() {
        let policy = OfflinePolicy::strict();

        let res1 = policy.validate_url("https://api.openai.com/v1/embeddings");
        assert!(res1.is_err());
        assert!(matches!(res1, Err(DomainError::OfflineViolation(_))));

        let res2 = policy.validate_url("http://192.168.1.100:8080");
        assert!(res2.is_err());

        let res3 = policy.validate_url("https://huggingface.co/models");
        assert!(res3.is_err());
    }

    #[test]
    fn test_air_gapped_policy_rejects_even_localhost() {
        let policy = OfflinePolicy::air_gapped();
        let res = policy.validate_url("http://localhost:8081/embedding");
        assert!(res.is_err());
        assert!(matches!(res, Err(DomainError::OfflineViolation(_))));
    }

    #[test]
    fn test_invalid_urls_rejected() {
        let policy = OfflinePolicy::strict();
        assert!(policy.validate_url("").is_err());
    }
}
