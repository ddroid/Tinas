use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// Represents a single version/snapshot of an image
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageVersion {
    pub id: String,           // Unique version ID
    pub timestamp: u64,         // Unix timestamp
    pub action_description: String, // Description of what changed
    pub image_data: Vec<u8>,    // PNG-encoded image data
    pub parent_version: Option<String>, // Previous version ID (for branching)
    pub is_saved: bool,         // Whether this version has been saved to pak
}

/// Tracks all versions for a single resource
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceVersions {
    pub resource_id: u16,
    pub pak_file: String,       // Which pak file this resource belongs to
    pub versions: Vec<ImageVersion>,
    pub current_version_id: Option<String>,
}

/// The main temp database that stores all versions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TempDb {
    db_path: PathBuf,
    resources: HashMap<String, ResourceVersions>, // key: "pak_file:resource_id"
}

impl TempDb {
    /// Create or load the temp database
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("pak_assets_manager");
        
        fs::create_dir_all(&cache_dir).ok();
        
        let db_path = cache_dir.join("versions.json");
        
        if db_path.exists() {
            match fs::read_to_string(&db_path) {
                Ok(contents) => {
                    match serde_json::from_str(&contents) {
                        Ok(db) => {
                            let mut db: TempDb = db;
                            db.db_path = db_path;
                            return db;
                        }
                        Err(e) => eprintln!("[TEMPDB] Failed to parse DB: {}", e),
                    }
                }
                Err(e) => eprintln!("[TEMPDB] Failed to read DB: {}", e),
            }
        }
        
        Self {
            db_path,
            resources: HashMap::new(),
        }
    }
    
    fn make_key(pak_file: &str, resource_id: u16) -> String {
        format!("{}:{}", pak_file, resource_id)
    }
    
    /// Save the database to disk
    pub fn save(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.db_path, json) {
                    eprintln!("[TEMPDB] Failed to save DB: {}", e);
                }
            }
            Err(e) => eprintln!("[TEMPDB] Failed to serialize DB: {}", e),
        }
    }
    
    /// Get or create version tracking for a resource
    pub fn get_or_create_resource(&mut self, pak_file: &str, resource_id: u16) -> &mut ResourceVersions {
        let key = Self::make_key(pak_file, resource_id);
        
        self.resources.entry(key).or_insert_with(|| ResourceVersions {
            resource_id,
            pak_file: pak_file.to_string(),
            versions: Vec::new(),
            current_version_id: None,
        })
    }
    
    /// Add a new version for a resource
    pub fn add_version(
        &mut self,
        pak_file: &str,
        resource_id: u16,
        action: &str,
        image_data: Vec<u8>,
    ) -> String {
        let key = Self::make_key(pak_file, resource_id);
        
        let resource = self.resources.entry(key.clone()).or_insert_with(|| ResourceVersions {
            resource_id,
            pak_file: pak_file.to_string(),
            versions: Vec::new(),
            current_version_id: None,
        });
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let version_id = format!("{}_{}_{}", resource_id, timestamp, resource.versions.len());
        
        let parent = resource.current_version_id.clone();
        
        let version = ImageVersion {
            id: version_id.clone(),
            timestamp,
            action_description: action.to_string(),
            image_data,
            parent_version: parent,
            is_saved: false,
        };
        
        resource.versions.push(version);
        resource.current_version_id = Some(version_id.clone());
        
        self.save();
        
        version_id
    }
    
    /// Get all versions for a resource
    pub fn get_versions(&self, pak_file: &str, resource_id: u16) -> Option<&Vec<ImageVersion>> {
        let key = Self::make_key(pak_file, resource_id);
        self.resources.get(&key).map(|r| &r.versions)
    }
    
    /// Get a specific version
    pub fn get_version(&self, pak_file: &str, resource_id: u16, version_id: &str) -> Option<&ImageVersion> {
        let key = Self::make_key(pak_file, resource_id);
        self.resources.get(&key)?.versions.iter().find(|v| v.id == version_id)
    }
    
    /// Mark all unsaved versions as saved for a resource
    pub fn mark_saved(&mut self, pak_file: &str, resource_id: u16) {
        let key = Self::make_key(pak_file, resource_id);
        if let Some(resource) = self.resources.get_mut(&key) {
            for version in &mut resource.versions {
                version.is_saved = true;
            }
            self.save();
        }
    }
    
    /// Revert to a specific version
    pub fn revert_to_version(&mut self, pak_file: &str, resource_id: u16, version_id: &str) -> Option<Vec<u8>> {
        let key = Self::make_key(pak_file, resource_id);
        
        if let Some(resource) = self.resources.get_mut(&key) {
            if let Some(version) = resource.versions.iter().find(|v| v.id == version_id).cloned() {
                // Create a new version for this revert action
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                let new_version_id = format!("{}_{}_{}_revert", resource_id, timestamp, resource.versions.len());
                
                let revert_version = ImageVersion {
                    id: new_version_id,
                    timestamp,
                    action_description: format!("Reverted to version {}", version_id),
                    image_data: version.image_data.clone(),
                    parent_version: resource.current_version_id.clone(),
                    is_saved: false,
                };
                
                resource.versions.push(revert_version);
                resource.current_version_id = Some(version_id.to_string());
                
                self.save();
                return Some(version.image_data);
            }
        }
        None
    }
    
    /// Clear all versions for a resource (when pak is closed)
    pub fn clear_resource(&mut self, pak_file: &str, resource_id: u16) {
        let key = Self::make_key(pak_file, resource_id);
        self.resources.remove(&key);
        self.save();
    }
    
    /// Get the current version ID for a resource
    pub fn get_current_version_id(&self, pak_file: &str, resource_id: u16) -> Option<String> {
        let key = Self::make_key(pak_file, resource_id);
        self.resources.get(&key)?.current_version_id.clone()
    }
    
    /// Check if resource has unsaved changes
    pub fn has_unsaved_changes(&self, pak_file: &str, resource_id: u16) -> bool {
        let key = Self::make_key(pak_file, resource_id);
        if let Some(resource) = self.resources.get(&key) {
            return resource.versions.iter().any(|v| !v.is_saved);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_tracking() {
        let mut db = TempDb::new();
        
        // Add versions
        let v1 = db.add_version("test.pak", 123, "Initial load", vec![1, 2, 3]);
        let v2 = db.add_version("test.pak", 123, "Brush stroke", vec![4, 5, 6]);
        
        assert_ne!(v1, v2);
        
        let versions = db.get_versions("test.pak", 123).unwrap();
        assert_eq!(versions.len(), 2);
        
        // Check unsaved
        assert!(db.has_unsaved_changes("test.pak", 123));
        
        // Mark saved
        db.mark_saved("test.pak", 123);
        assert!(!db.has_unsaved_changes("test.pak", 123));
    }
}
