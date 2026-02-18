//! Type definitions for container management

use std::fmt;

/// Status of a container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStatus {
    /// Container is running
    Running,
    /// Container exists but is stopped
    Stopped,
    /// Container does not exist
    NotFound,
}

impl fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerStatus::Running => write!(f, "Running"),
            ContainerStatus::Stopped => write!(f, "Stopped"),
            ContainerStatus::NotFound => write!(f, "Not Found"),
        }
    }
}

/// Information about a container
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInfo {
    /// Container name (e.g., "icitadel-dev-postgres")
    pub name: String,
    /// Current status
    pub status: ContainerStatus,
    /// Port mappings (e.g., "0.0.0.0:5432->5432/tcp")
    pub ports: String,
}

impl ContainerInfo {
    /// Create a new ContainerInfo
    pub fn new(name: String, status: ContainerStatus, ports: String) -> Self {
        Self {
            name,
            status,
            ports,
        }
    }

    /// Check if the container is running
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
    }

    /// Extract the database type from container name
    ///
    /// # Example
    /// ```
    /// use rusty_app::container::{ContainerInfo, ContainerStatus};
    ///
    /// let info = ContainerInfo::new(
    ///     "icitadel-dev-postgres".to_string(),
    ///     ContainerStatus::Running,
    ///     "5432".to_string(),
    /// );
    /// assert_eq!(info.database_type(), Some("postgres"));
    /// ```
    pub fn database_type(&self) -> Option<&str> {
        if !self.name.starts_with("icitadel-dev-") {
            return None;
        }

        let db_type = self.name.strip_prefix("icitadel-dev-")?;
        Some(db_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_display() {
        assert_eq!(ContainerStatus::Running.to_string(), "Running");
        assert_eq!(ContainerStatus::Stopped.to_string(), "Stopped");
        assert_eq!(ContainerStatus::NotFound.to_string(), "Not Found");
    }

    #[test]
    fn test_container_status_equality() {
        assert_eq!(ContainerStatus::Running, ContainerStatus::Running);
        assert_ne!(ContainerStatus::Running, ContainerStatus::Stopped);
    }

    #[test]
    fn test_container_info_new() {
        let info = ContainerInfo::new(
            "icitadel-dev-postgres".to_string(),
            ContainerStatus::Running,
            "5432".to_string(),
        );

        assert_eq!(info.name, "icitadel-dev-postgres");
        assert_eq!(info.status, ContainerStatus::Running);
        assert_eq!(info.ports, "5432");
    }

    #[test]
    fn test_container_info_is_running() {
        let running = ContainerInfo::new(
            "test".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert!(running.is_running());

        let stopped = ContainerInfo::new(
            "test".to_string(),
            ContainerStatus::Stopped,
            "".to_string(),
        );
        assert!(!stopped.is_running());
    }

    #[test]
    fn test_container_info_database_type() {
        let postgres = ContainerInfo::new(
            "icitadel-dev-postgres".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(postgres.database_type(), Some("postgres"));

        let mysql = ContainerInfo::new(
            "icitadel-dev-mysql".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(mysql.database_type(), Some("mysql"));

        let mongodb = ContainerInfo::new(
            "icitadel-dev-mongodb".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(mongodb.database_type(), Some("mongodb"));

        let mssql = ContainerInfo::new(
            "icitadel-dev-mssql".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(mssql.database_type(), Some("mssql"));

        let oracle = ContainerInfo::new(
            "icitadel-dev-oracle".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(oracle.database_type(), Some("oracle"));

        let invalid = ContainerInfo::new(
            "other-container".to_string(),
            ContainerStatus::Running,
            "".to_string(),
        );
        assert_eq!(invalid.database_type(), None);
    }

    #[test]
    fn test_container_info_clone() {
        let info = ContainerInfo::new(
            "test".to_string(),
            ContainerStatus::Running,
            "5432".to_string(),
        );
        let cloned = info.clone();

        assert_eq!(info, cloned);
    }
}
