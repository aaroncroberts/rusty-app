//! Container management service for local database containers.
//!
//! Provides lifecycle management for icitadel-dev-* containers using podman/docker-compose.

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

mod types;
pub use types::{ContainerInfo, ContainerStatus};

/// Errors that can occur during container operations
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Podman command not found. Please install podman.")]
    PodmanNotAvailable,

    #[error("Container '{0}' not found")]
    ContainerNotFound(String),

    #[error("Container operation failed: {0}")]
    OperationFailed(String),

    #[error("Compose file not found at {0}")]
    ComposeFileNotFound(PathBuf),

    #[error("Failed to execute command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse command output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ContainerError>;

/// Container manager for icitadel-dev-* database containers
///
/// Manages lifecycle operations (start/stop/list) for local development containers
/// defined in rusty-data/podman-compose.yml.
pub struct ContainerManager {
    /// Path to the rusty-data directory containing podman-compose.yml
    compose_dir: PathBuf,
}

impl ContainerManager {
    /// Create a new ContainerManager
    ///
    /// # Arguments
    /// * `compose_dir` - Path to directory containing podman-compose.yml (typically "rusty-data")
    ///
    /// # Example
    /// ```no_run
    /// use rusty_app::container::ContainerManager;
    /// use std::path::Path;
    ///
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// ```
    pub fn new(compose_dir: impl Into<PathBuf>) -> Self {
        Self {
            compose_dir: compose_dir.into(),
        }
    }

    /// Check if podman command is available
    ///
    /// # Example
    /// ```no_run
    /// use rusty_app::container::ContainerManager;
    /// use std::path::Path;
    ///
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// if manager.is_podman_available() {
    ///     println!("Podman is installed");
    /// }
    /// ```
    pub fn is_podman_available(&self) -> bool {
        Command::new("podman")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Verify compose file exists
    fn check_compose_file(&self) -> Result<PathBuf> {
        let compose_path = self.compose_dir.join("podman-compose.yml");

        if !compose_path.exists() {
            return Err(ContainerError::ComposeFileNotFound(compose_path));
        }

        Ok(compose_path)
    }

    /// List all icitadel-dev-* containers and their status
    ///
    /// # Returns
    /// A vector of ContainerInfo structs containing container details
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::ContainerManager;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// let containers = manager.list_containers()?;
    /// for container in containers {
    ///     println!("{}: {}", container.name, container.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        let output = Command::new("podman")
            .args(&[
                "ps",
                "-a",
                "--filter", "name=icitadel-dev",
                "--format", "{{.Names}}\t{{.Status}}\t{{.Ports}}",
            ])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::CommandFailed(error.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut containers = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let status_str = parts[1];
                let ports = if parts.len() >= 3 {
                    parts[2].to_string()
                } else {
                    String::new()
                };

                let status = if status_str.to_lowercase().starts_with("up") {
                    ContainerStatus::Running
                } else {
                    ContainerStatus::Stopped
                };

                containers.push(ContainerInfo {
                    name,
                    status,
                    ports,
                });
            }
        }

        Ok(containers)
    }

    /// Get the status of a specific container
    ///
    /// # Arguments
    /// * `container_name` - Name of the container (e.g., "icitadel-dev-postgres")
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::{ContainerManager, ContainerStatus};
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// let status = manager.status("icitadel-dev-postgres")?;
    /// match status {
    ///     ContainerStatus::Running => println!("Container is running"),
    ///     ContainerStatus::Stopped => println!("Container is stopped"),
    ///     ContainerStatus::NotFound => println!("Container not found"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn status(&self, container_name: &str) -> Result<ContainerStatus> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        let output = Command::new("podman")
            .args(&[
                "ps",
                "-a",
                "--filter", &format!("name=^{}$", container_name),
                "--format", "{{.Status}}",
            ])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(ContainerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let status_str = stdout.trim();

        if status_str.is_empty() {
            Ok(ContainerStatus::NotFound)
        } else if status_str.to_lowercase().starts_with("up") {
            Ok(ContainerStatus::Running)
        } else {
            Ok(ContainerStatus::Stopped)
        }
    }

    /// Start a specific container using podman-compose
    ///
    /// # Arguments
    /// * `container_name` - Name of the container (e.g., "icitadel-dev-postgres")
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::ContainerManager;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// manager.start("icitadel-dev-postgres")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start(&self, container_name: &str) -> Result<()> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        self.check_compose_file()?;

        let output = Command::new("podman-compose")
            .current_dir(&self.compose_dir)
            .args(&["up", "-d", container_name])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::OperationFailed(format!(
                "Failed to start container '{}': {}",
                container_name, error
            )));
        }

        Ok(())
    }

    /// Stop a specific container using podman-compose
    ///
    /// # Arguments
    /// * `container_name` - Name of the container (e.g., "icitadel-dev-postgres")
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::ContainerManager;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// manager.stop("icitadel-dev-postgres")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stop(&self, container_name: &str) -> Result<()> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        self.check_compose_file()?;

        let output = Command::new("podman-compose")
            .current_dir(&self.compose_dir)
            .args(&["stop", container_name])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::OperationFailed(format!(
                "Failed to stop container '{}': {}",
                container_name, error
            )));
        }

        Ok(())
    }

    /// Start all icitadel-dev-* containers
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::ContainerManager;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// manager.start_all()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_all(&self) -> Result<()> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        self.check_compose_file()?;

        let output = Command::new("podman-compose")
            .current_dir(&self.compose_dir)
            .args(&["up", "-d"])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::OperationFailed(format!(
                "Failed to start all containers: {}",
                error
            )));
        }

        Ok(())
    }

    /// Stop all icitadel-dev-* containers
    ///
    /// # Example
    /// ```no_run
    /// # use rusty_app::container::ContainerManager;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = ContainerManager::new(Path::new("rusty-data"));
    /// manager.stop_all()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stop_all(&self) -> Result<()> {
        if !self.is_podman_available() {
            return Err(ContainerError::PodmanNotAvailable);
        }

        self.check_compose_file()?;

        let output = Command::new("podman-compose")
            .current_dir(&self.compose_dir)
            .args(&["down"])
            .output()
            .map_err(|e| ContainerError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ContainerError::OperationFailed(format!(
                "Failed to stop all containers: {}",
                error
            )));
        }

        Ok(())
    }
}

impl Default for ContainerManager {
    fn default() -> Self {
        Self::new("rusty-data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_manager_creation() {
        let manager = ContainerManager::new("rusty-data");
        assert_eq!(manager.compose_dir, PathBuf::from("rusty-data"));
    }

    #[test]
    fn test_container_manager_default() {
        let manager = ContainerManager::default();
        assert_eq!(manager.compose_dir, PathBuf::from("rusty-data"));
    }

    #[test]
    fn test_is_podman_available() {
        let manager = ContainerManager::new("rusty-data");
        // This test will succeed if podman is installed, fail otherwise
        // We just verify it doesn't panic
        let _result = manager.is_podman_available();
    }

    #[test]
    fn test_check_compose_file_missing() {
        let manager = ContainerManager::new("nonexistent-directory");
        let result = manager.check_compose_file();
        assert!(result.is_err());

        if let Err(ContainerError::ComposeFileNotFound(path)) = result {
            assert!(path.to_string_lossy().contains("nonexistent-directory"));
        } else {
            panic!("Expected ComposeFileNotFound error");
        }
    }

    #[test]
    fn test_status_podman_not_available() {
        // This test assumes podman is not available, which may not be true in all environments
        // The test is here to verify the error handling logic
        let manager = ContainerManager::new("rusty-data");

        if !manager.is_podman_available() {
            let result = manager.status("icitadel-dev-postgres");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ContainerError::PodmanNotAvailable));
        }
    }

    // Integration tests (require actual podman setup)
    #[test]
    #[ignore = "Requires podman and running containers"]
    fn test_list_containers_integration() {
        let manager = ContainerManager::new("rusty-data");
        let result = manager.list_containers();
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires podman and containers"]
    fn test_status_integration() {
        let manager = ContainerManager::new("rusty-data");
        let result = manager.status("icitadel-dev-postgres");
        assert!(result.is_ok());
        // Status should be Running, Stopped, or NotFound
        let status = result.unwrap();
        assert!(matches!(
            status,
            ContainerStatus::Running | ContainerStatus::Stopped | ContainerStatus::NotFound
        ));
    }

    #[test]
    #[ignore = "Requires podman, modifies container state"]
    fn test_start_stop_integration() {
        let manager = ContainerManager::new("rusty-data");

        // Start container
        let start_result = manager.start("icitadel-dev-postgres");
        assert!(start_result.is_ok());

        // Wait a moment for container to start
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Check status
        let status = manager.status("icitadel-dev-postgres").unwrap();
        assert_eq!(status, ContainerStatus::Running);

        // Stop container
        let stop_result = manager.stop("icitadel-dev-postgres");
        assert!(stop_result.is_ok());

        // Wait a moment for container to stop
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Check status
        let status = manager.status("icitadel-dev-postgres").unwrap();
        assert_eq!(status, ContainerStatus::Stopped);
    }
}
