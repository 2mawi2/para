use crate::utils::{ParaError, Result};

#[cfg(test)]
use super::mock::MockDockerClient;

// TODO: Connect to CLI in next phase
pub const AUTH_BASE_CONTAINER_NAME: &str = "para-auth-base";
pub const AUTH_VOLUME_PREFIX: &str = "para-auth-claude";
pub const CONFIG_VOLUME_PREFIX: &str = "para-auth-ide-config";

// TODO: Connect to CLI in next phase
#[derive(Clone)]
pub struct DockerAuthManager {
    #[cfg(test)]
    mock_client: Option<MockDockerClient>,
    user_id: String,
}

impl DockerAuthManager {
    pub fn new(user_id: String) -> Self {
        Self {
            #[cfg(test)]
            mock_client: Some(MockDockerClient::new()),
            user_id,
        }
    }

    #[cfg(test)]
    pub fn with_mock_client(mock_client: MockDockerClient, user_id: String) -> Self {
        Self {
            mock_client: Some(mock_client),
            user_id,
        }
    }

    pub fn claude_volume_name(&self) -> String {
        format!("{}-{}", AUTH_VOLUME_PREFIX, self.user_id)
    }

    pub fn config_volume_name(&self) -> String {
        format!("{}-{}", CONFIG_VOLUME_PREFIX, self.user_id)
    }

    pub fn ensure_auth_volumes(&self) -> Result<()> {
        #[cfg(test)]
        {
            if let Some(client) = &self.mock_client {
                // Create Claude auth volume
                client
                    .create_volume(&self.claude_volume_name())
                    .map_err(|e| {
                        ParaError::docker_error(format!(
                            "Failed to create Claude auth volume: {}",
                            e
                        ))
                    })?;

                // Create IDE config volume
                client
                    .create_volume(&self.config_volume_name())
                    .map_err(|e| {
                        ParaError::docker_error(format!(
                            "Failed to create IDE config volume: {}",
                            e
                        ))
                    })?;

                return Ok(());
            }
        }

        // TODO: Real Docker implementation
        Err(ParaError::docker_error(
            "Real Docker implementation not yet available".to_string(),
        ))
    }

    pub fn ensure_auth_base_container(&self) -> Result<()> {
        #[cfg(test)]
        {
            if let Some(client) = &self.mock_client {
                if client.container_exists(AUTH_BASE_CONTAINER_NAME) {
                    return Ok(());
                }

                // Create base container with auth volumes mounted
                let claude_volume = client
                    .create_volume(&self.claude_volume_name())
                    .map_err(ParaError::docker_error)?;
                let config_volume = client
                    .create_volume(&self.config_volume_name())
                    .map_err(ParaError::docker_error)?;

                client
                    .create_container(
                        AUTH_BASE_CONTAINER_NAME,
                        vec![claude_volume, config_volume],
                        vec![],
                    )
                    .map_err(|e| {
                        ParaError::docker_error(format!(
                            "Failed to create auth base container: {}",
                            e
                        ))
                    })?;

                return Ok(());
            }
        }

        // TODO: Real Docker implementation
        Err(ParaError::docker_error(
            "Real Docker implementation not yet available".to_string(),
        ))
    }

    pub fn get_volume_mount_args(&self) -> Vec<String> {
        vec![
            "--volumes-from".to_string(),
            AUTH_BASE_CONTAINER_NAME.to_string(),
        ]
    }

    pub fn cleanup_auth_volumes(&self) -> Result<()> {
        #[cfg(test)]
        {
            if let Some(client) = &self.mock_client {
                // Stop and remove base container if it exists
                if client.container_exists(AUTH_BASE_CONTAINER_NAME) {
                    if let Some(container) = client.get_container(AUTH_BASE_CONTAINER_NAME) {
                        if container.running {
                            client
                                .stop_container(AUTH_BASE_CONTAINER_NAME)
                                .map_err(ParaError::docker_error)?;
                        }
                    }
                    client
                        .remove_container(AUTH_BASE_CONTAINER_NAME)
                        .map_err(ParaError::docker_error)?;
                }

                // Remove volumes
                if client.volume_exists(&self.claude_volume_name()) {
                    client
                        .remove_volume(&self.claude_volume_name())
                        .map_err(|e| {
                            ParaError::docker_error(format!(
                                "Failed to remove Claude auth volume: {}",
                                e
                            ))
                        })?;
                }

                if client.volume_exists(&self.config_volume_name()) {
                    client
                        .remove_volume(&self.config_volume_name())
                        .map_err(|e| {
                            ParaError::docker_error(format!(
                                "Failed to remove IDE config volume: {}",
                                e
                            ))
                        })?;
                }

                return Ok(());
            }
        }

        // TODO: Real Docker implementation
        Err(ParaError::docker_error(
            "Real Docker implementation not yet available".to_string(),
        ))
    }

    pub fn check_volumes_exist(&self) -> Result<bool> {
        #[cfg(test)]
        {
            if let Some(client) = &self.mock_client {
                let claude_exists = client.volume_exists(&self.claude_volume_name());
                let config_exists = client.volume_exists(&self.config_volume_name());
                return Ok(claude_exists && config_exists);
            }
        }

        // TODO: Real Docker implementation
        Err(ParaError::docker_error(
            "Real Docker implementation not yet available".to_string(),
        ))
    }

    pub fn is_first_time_setup(&self) -> Result<bool> {
        Ok(!self.check_volumes_exist()?)
    }

    pub fn get_permission_mapping(&self) -> Result<(u32, u32)> {
        // TODO: Get actual UID/GID mapping between host and container
        // For now, return para user's expected UID/GID in container
        Ok((1000, 1000)) // Standard para user in container
    }

    pub fn setup_initial_auth(&self, ide_name: &str) -> Result<()> {
        println!("🔐 First-time authentication setup for {}", ide_name);
        println!("   This will persist your authentication across container sessions.");

        // Ensure volumes exist
        self.ensure_auth_volumes()?;

        // Create base container
        self.ensure_auth_base_container()?;

        println!("✅ Authentication volumes created successfully!");
        println!("   - Claude auth: {}", self.claude_volume_name());
        println!("   - IDE config: {}", self.config_volume_name());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_names() {
        let manager = DockerAuthManager::new("1000".to_string());
        assert_eq!(manager.claude_volume_name(), "para-auth-claude-1000");
        assert_eq!(manager.config_volume_name(), "para-auth-ide-config-1000");
    }

    #[test]
    fn test_ensure_auth_volumes() {
        let client = MockDockerClient::new();
        let manager = DockerAuthManager::with_mock_client(client.clone(), "1000".to_string());

        // Initially no volumes
        assert!(!manager.check_volumes_exist().unwrap());
        assert!(manager.is_first_time_setup().unwrap());

        // Create volumes
        assert!(manager.ensure_auth_volumes().is_ok());

        // Verify volumes exist
        assert!(manager.check_volumes_exist().unwrap());
        assert!(!manager.is_first_time_setup().unwrap());
        assert!(client.volume_exists("para-auth-claude-1000"));
        assert!(client.volume_exists("para-auth-ide-config-1000"));
    }

    #[test]
    fn test_ensure_auth_base_container() {
        let client = MockDockerClient::new();
        let manager = DockerAuthManager::with_mock_client(client.clone(), "1000".to_string());

        // Create base container (will also create volumes)
        assert!(manager.ensure_auth_base_container().is_ok());

        // Verify container exists
        assert!(client.container_exists(AUTH_BASE_CONTAINER_NAME));

        // Verify volumes were created
        assert!(client.volume_exists("para-auth-claude-1000"));
        assert!(client.volume_exists("para-auth-ide-config-1000"));
    }

    #[test]
    fn test_cleanup_auth_volumes() {
        let client = MockDockerClient::new();
        let manager = DockerAuthManager::with_mock_client(client.clone(), "1000".to_string());

        // Setup
        assert!(manager.ensure_auth_volumes().is_ok());
        assert!(manager.ensure_auth_base_container().is_ok());

        // Cleanup
        assert!(manager.cleanup_auth_volumes().is_ok());

        // Verify everything is gone
        assert!(!client.volume_exists("para-auth-claude-1000"));
        assert!(!client.volume_exists("para-auth-ide-config-1000"));
        assert!(!client.container_exists(AUTH_BASE_CONTAINER_NAME));
    }

    #[test]
    fn test_volume_mount_args() {
        let manager = DockerAuthManager::new("1000".to_string());
        let args = manager.get_volume_mount_args();
        assert_eq!(args, vec!["--volumes-from", AUTH_BASE_CONTAINER_NAME]);
    }

    #[test]
    fn test_permission_mapping() {
        let manager = DockerAuthManager::new("1000".to_string());
        let (uid, gid) = manager.get_permission_mapping().unwrap();
        assert_eq!(uid, 1000);
        assert_eq!(gid, 1000);
    }

    #[test]
    fn test_setup_initial_auth() {
        let client = MockDockerClient::new();
        let manager = DockerAuthManager::with_mock_client(client.clone(), "1000".to_string());

        // Initial setup
        assert!(manager.setup_initial_auth("Claude Code").is_ok());

        // Verify everything was created
        assert!(client.volume_exists("para-auth-claude-1000"));
        assert!(client.volume_exists("para-auth-ide-config-1000"));
        assert!(client.container_exists(AUTH_BASE_CONTAINER_NAME));
    }
}
