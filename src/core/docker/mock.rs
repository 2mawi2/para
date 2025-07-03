use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct MockVolume {
    pub name: String,
    pub mount_point: String,
    pub exists: bool,
}

#[derive(Debug, Clone)]
pub struct MockContainer {
    pub id: String,
    pub name: String,
    pub volumes: Vec<MockVolume>,
    pub volumes_from: Vec<String>,
    pub running: bool,
}

#[derive(Clone)]
pub struct MockDockerClient {
    volumes: Arc<Mutex<HashMap<String, MockVolume>>>,
    containers: Arc<Mutex<HashMap<String, MockContainer>>>,
}

impl Default for MockDockerClient {
    fn default() -> Self {
        Self {
            volumes: Arc::new(Mutex::new(HashMap::new())),
            containers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl MockDockerClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_volume(&self, name: &str) -> Result<MockVolume, String> {
        let mut volumes = self.volumes.lock().unwrap();

        if volumes.contains_key(name) {
            return Ok(volumes[name].clone());
        }

        let volume = MockVolume {
            name: name.to_string(),
            mount_point: format!("/var/lib/docker/volumes/{name}"),
            exists: true,
        };

        volumes.insert(name.to_string(), volume.clone());
        Ok(volume)
    }

    pub fn volume_exists(&self, name: &str) -> bool {
        let volumes = self.volumes.lock().unwrap();
        volumes.contains_key(name)
    }

    pub fn remove_volume(&self, name: &str) -> Result<(), String> {
        let mut volumes = self.volumes.lock().unwrap();

        if !volumes.contains_key(name) {
            return Err(format!("Volume '{name}' not found"));
        }

        // Check if volume is in use by any container
        let containers = self.containers.lock().unwrap();
        for container in containers.values() {
            if container.volumes.iter().any(|v| v.name == name) {
                return Err(format!(
                    "Volume '{}' is in use by container '{}'",
                    name, container.name
                ));
            }
        }

        volumes.remove(name);
        Ok(())
    }

    pub fn create_container(
        &self,
        name: &str,
        volumes: Vec<MockVolume>,
        volumes_from: Vec<String>,
    ) -> Result<MockContainer, String> {
        let mut containers = self.containers.lock().unwrap();

        if containers.contains_key(name) {
            return Err(format!("Container '{name}' already exists"));
        }

        // Verify volumes exist
        let volume_guard = self.volumes.lock().unwrap();
        for volume in &volumes {
            if !volume_guard.contains_key(&volume.name) {
                return Err(format!("Volume '{}' does not exist", volume.name));
            }
        }
        drop(volume_guard);

        // Verify volumes_from containers exist
        for container_name in &volumes_from {
            if !containers.contains_key(container_name) {
                return Err(format!(
                    "Container '{container_name}' to inherit volumes from does not exist"
                ));
            }
        }

        let container = MockContainer {
            id: format!("mock-{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            volumes,
            volumes_from,
            running: false,
        };

        containers.insert(name.to_string(), container.clone());
        Ok(container)
    }

    pub fn container_exists(&self, name: &str) -> bool {
        let containers = self.containers.lock().unwrap();
        containers.contains_key(name)
    }

    pub fn start_container(&self, name: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().unwrap();

        match containers.get_mut(name) {
            Some(container) => {
                container.running = true;
                Ok(())
            }
            None => Err(format!("Container '{name}' not found")),
        }
    }

    pub fn stop_container(&self, name: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().unwrap();

        match containers.get_mut(name) {
            Some(container) => {
                container.running = false;
                Ok(())
            }
            None => Err(format!("Container '{name}' not found")),
        }
    }

    pub fn remove_container(&self, name: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().unwrap();

        if !containers.contains_key(name) {
            return Err(format!("Container '{name}' not found"));
        }

        let container = &containers[name];
        if container.running {
            return Err(format!("Container '{name}' is running"));
        }

        containers.remove(name);
        Ok(())
    }

    pub fn get_container(&self, name: &str) -> Option<MockContainer> {
        let containers = self.containers.lock().unwrap();
        containers.get(name).cloned()
    }

    pub fn list_volumes(&self) -> Vec<MockVolume> {
        let volumes = self.volumes.lock().unwrap();
        volumes.values().cloned().collect()
    }

    pub fn list_containers(&self) -> Vec<MockContainer> {
        let containers = self.containers.lock().unwrap();
        containers.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_volume_operations() {
        let client = MockDockerClient::new();

        // Create volume
        let volume = client.create_volume("test-volume").unwrap();
        assert_eq!(volume.name, "test-volume");
        assert!(volume.exists);
        assert_eq!(volume.mount_point, "/var/lib/docker/volumes/test-volume");

        // Check existence
        assert!(client.volume_exists("test-volume"));
        assert!(!client.volume_exists("non-existent"));

        // Create duplicate (should return existing)
        let duplicate = client.create_volume("test-volume").unwrap();
        assert_eq!(duplicate.name, volume.name);

        // Remove volume
        assert!(client.remove_volume("test-volume").is_ok());
        assert!(!client.volume_exists("test-volume"));
    }

    #[test]
    fn test_mock_container_operations() {
        let client = MockDockerClient::new();

        // Create volume first
        let volume = client.create_volume("test-volume").unwrap();

        // Create container
        let container = client
            .create_container("test-container", vec![volume], vec![])
            .unwrap();
        assert_eq!(container.name, "test-container");
        assert!(!container.running);
        assert!(container.id.starts_with("mock-"));
        assert_eq!(container.volumes_from.len(), 0);

        // Check existence
        assert!(client.container_exists("test-container"));

        // Start container
        assert!(client.start_container("test-container").is_ok());
        let container = client.get_container("test-container").unwrap();
        assert!(container.running);

        // Cannot remove running container
        assert!(client.remove_container("test-container").is_err());

        // Stop and remove
        assert!(client.stop_container("test-container").is_ok());
        assert!(client.remove_container("test-container").is_ok());
        assert!(!client.container_exists("test-container"));
    }

    #[test]
    fn test_volume_in_use_protection() {
        let client = MockDockerClient::new();

        // Create volume and container using it
        let volume = client.create_volume("test-volume").unwrap();
        client
            .create_container("test-container", vec![volume], vec![])
            .unwrap();

        // Cannot remove volume in use
        assert!(client.remove_volume("test-volume").is_err());

        // Remove container first, then volume
        assert!(client.remove_container("test-container").is_ok());
        assert!(client.remove_volume("test-volume").is_ok());
    }

    #[test]
    fn test_list_operations() {
        let client = MockDockerClient::new();

        // Initially empty
        assert_eq!(client.list_volumes().len(), 0);
        assert_eq!(client.list_containers().len(), 0);

        // Create some volumes and containers
        client.create_volume("vol1").unwrap();
        client.create_volume("vol2").unwrap();

        let vol = client.create_volume("vol3").unwrap();
        client.create_container("cont1", vec![vol], vec![]).unwrap();

        // Check lists
        assert_eq!(client.list_volumes().len(), 3);
        assert_eq!(client.list_containers().len(), 1);
    }
}
