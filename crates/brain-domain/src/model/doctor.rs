//! Modelos de Dominio para Observabilidad y Diagnóstico de Salud del Sistema (SRS §35).
//!
//! Estructuras puras para recolectar, categorizar y reportar el estado de salud de
//! los subsistemas de Local Brain (PostgreSQL, pgvector, llama.cpp, configuraciones y jobs).

use serde::{Deserialize, Serialize};

/// Estado de un chequeo o verificación de salud individual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// El componente opera de forma nominal y óptima.
    Ok,
    /// El componente presenta una advertencia no crítica o recomendación de mejora.
    Warn,
    /// El componente ha fallado o se encuentra inoperativo, impidiendo el funcionamiento correcto.
    Fail,
}

impl CheckStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ok => "🟢",
            Self::Warn => "🟡",
            Self::Fail => "🔴",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

/// Resultado de una verificación de salud de un subsistema o componente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Categoría del chequeo (e.g., "Entorno", "Persistencia", "Embeddings", "Jobs").
    pub category: String,
    /// Nombre descriptivo del chequeo (e.g., "Conectividad PostgreSQL").
    pub name: String,
    /// Estado del resultado del chequeo.
    pub status: CheckStatus,
    /// Mensaje descriptivo con detalles técnicos del estado.
    pub message: String,
    /// Comando o acción sugerida para subsanar el problema si status es Warn o Fail.
    pub remedy: Option<String>,
    /// Latencia medida de la operación en milisegundos si aplica.
    pub latency_ms: Option<f64>,
}

impl HealthCheck {
    pub fn ok(
        category: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: CheckStatus::Ok,
            message: message.into(),
            remedy: None,
            latency_ms: None,
        }
    }

    pub fn ok_with_latency(
        category: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
        latency_ms: f64,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: CheckStatus::Ok,
            message: message.into(),
            remedy: None,
            latency_ms: Some(latency_ms),
        }
    }

    pub fn warn(
        category: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            remedy: Some(remedy.into()),
            latency_ms: None,
        }
    }

    pub fn fail(
        category: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            remedy: Some(remedy.into()),
            latency_ms: None,
        }
    }
}

/// Reporte consolidado de diagnóstico generado por `brain doctor`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Versión del núcleo de Local Brain.
    pub core_version: String,
    /// Sistema operativo donde se ejecuta.
    pub os: String,
    /// Lista de verificaciones ejecutadas.
    pub checks: Vec<HealthCheck>,
}

impl DoctorReport {
    pub fn new(core_version: impl Into<String>, os: impl Into<String>) -> Self {
        Self {
            core_version: core_version.into(),
            os: os.into(),
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, check: HealthCheck) {
        self.checks.push(check);
    }

    /// Retorna `true` si ningún chequeo falló con `Fail`.
    pub fn is_healthy(&self) -> bool {
        !self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }

    pub fn fail_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count()
    }

    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count()
    }

    pub fn ok_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Ok)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_report_aggregation() {
        let mut report = DoctorReport::new("0.1.0", "linux");
        assert!(report.is_healthy());
        assert_eq!(report.ok_count(), 0);

        report.add_check(HealthCheck::ok("DB", "PostgreSQL", "Connected"));
        report.add_check(HealthCheck::warn(
            "Config",
            "Limits",
            "High limit",
            "Lower limit in brain.toml",
        ));

        assert!(report.is_healthy());
        assert_eq!(report.ok_count(), 1);
        assert_eq!(report.warn_count(), 1);
        assert_eq!(report.fail_count(), 0);

        report.add_check(HealthCheck::fail(
            "LLM",
            "llama.cpp",
            "Unreachable",
            "docker compose up -d",
        ));
        assert!(!report.is_healthy());
        assert_eq!(report.fail_count(), 1);
    }

    #[test]
    fn test_check_status_icons_and_labels() {
        assert_eq!(CheckStatus::Ok.icon(), "🟢");
        assert_eq!(CheckStatus::Warn.icon(), "🟡");
        assert_eq!(CheckStatus::Fail.icon(), "🔴");
        assert_eq!(CheckStatus::Ok.label(), "OK");
        assert_eq!(CheckStatus::Warn.label(), "WARN");
        assert_eq!(CheckStatus::Fail.label(), "FAIL");
    }
}
