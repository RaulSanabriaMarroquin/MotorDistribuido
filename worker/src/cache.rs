use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum memory size in MB before spilling to disk
    pub max_memory_mb: f64,
    /// Directory for spill files
    pub spill_dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 100.0, // Default: 100 MB
            spill_dir: PathBuf::from("cache_spill"),
        }
    }
}

/// Cache entry for a partition
#[derive(Debug, Clone)]
pub enum CacheEntry {
    /// Data in memory
    InMemory(Vec<serde_json::Value>),
    /// Data spilled to disk
    OnDisk {
        path: PathBuf,
        record_count: usize,
    },
}

/// Partition cache with spill to disk
pub struct PartitionCache {
    config: CacheConfig,
    partitions: Arc<RwLock<HashMap<u32, CacheEntry>>>,
    current_memory_mb: Arc<RwLock<f64>>,
}

impl PartitionCache {
    pub fn new(config: CacheConfig) -> Result<Self, String> {
        // Create spill directory
        fs::create_dir_all(&config.spill_dir)
            .map_err(|e| format!("Failed to create spill directory: {}", e))?;

        Ok(Self {
            config,
            partitions: Arc::new(RwLock::new(HashMap::new())),
            current_memory_mb: Arc::new(RwLock::new(0.0)),
        })
    }

    /// Add records to a partition
    pub async fn add_records(
        &self,
        partition: u32,
        records: Vec<serde_json::Value>,
    ) -> Result<(), String> {
        let mut partitions = self.partitions.write().await;
        let mut current_memory = self.current_memory_mb.write().await;

        // Estimate memory usage (rough approximation: JSON string size)
        let records_size_mb = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default().len())
            .sum::<usize>() as f64
            / (1024.0 * 1024.0);

        // Check if we need to spill
        if *current_memory + records_size_mb > self.config.max_memory_mb {
            debug!(
                current_memory_mb = *current_memory,
                new_size_mb = records_size_mb,
                max_memory_mb = self.config.max_memory_mb,
                "Memory threshold exceeded, spilling to disk"
            );

            // Spill existing in-memory partitions to disk
            drop(current_memory); // Release lock before calling spill_partitions
            self.spill_partitions(&mut partitions).await?;
            *self.current_memory_mb.write().await = 0.0; // Reset after spill
            current_memory = self.current_memory_mb.write().await;
        }

        // Add records to partition
        match partitions.get_mut(&partition) {
            Some(CacheEntry::InMemory(existing)) => {
                existing.extend(records);
            }
            Some(CacheEntry::OnDisk { path, record_count }) => {
                // Append to spilled file
                self.append_to_spill_file(path, &records, *record_count)?;
                *record_count += records.len();
            }
            None => {
                // New partition
                if *current_memory + records_size_mb > self.config.max_memory_mb {
                    // Spill immediately
                    let spill_path = self.config.spill_dir.join(format!("partition_{}.jsonl", partition));
                    self.write_to_spill_file(&spill_path, &records)?;
                    partitions.insert(
                        partition,
                        CacheEntry::OnDisk {
                            path: spill_path,
                            record_count: records.len(),
                        },
                    );
                } else {
                    partitions.insert(partition, CacheEntry::InMemory(records));
                    *current_memory += records_size_mb;
                }
            }
        }

        Ok(())
    }

    /// Get all records for a partition (loads from disk if needed)
    pub async fn get_records(&self, partition: u32) -> Result<Vec<serde_json::Value>, String> {
        let spill_path = {
            let partitions = self.partitions.read().await;
            match partitions.get(&partition) {
                Some(CacheEntry::InMemory(records)) => return Ok(records.clone()),
                Some(CacheEntry::OnDisk { path, .. }) => path.clone(),
                None => return Ok(Vec::new()),
            }
        };
        
        // Load from disk (lock released)
        self.load_from_spill_file(&spill_path)
    }

    /// Clear a partition
    pub async fn clear_partition(&self, partition: u32) {
        let mut partitions = self.partitions.write().await;
        if let Some(entry) = partitions.remove(&partition) {
            if let CacheEntry::OnDisk { path, .. } = entry {
                // Clean up spill file
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// Clear all partitions
    pub async fn clear_all(&self) {
        let mut partitions = self.partitions.write().await;
        for entry in partitions.values() {
            if let CacheEntry::OnDisk { path, .. } = entry {
                let _ = fs::remove_file(path);
            }
        }
        partitions.clear();
        *self.current_memory_mb.write().await = 0.0;
    }

    /// Spill in-memory partitions to disk
    async fn spill_partitions(
        &self,
        partitions: &mut HashMap<u32, CacheEntry>,
    ) -> Result<(), String> {
        let mut to_spill = Vec::new();

        // Find partitions to spill
        for (partition, entry) in partitions.iter() {
            if let CacheEntry::InMemory(records) = entry {
                if !records.is_empty() {
                    to_spill.push((*partition, records.clone()));
                }
            }
        }

        // Spill each partition
        for (partition, records) in to_spill {
            let spill_path = self.config.spill_dir.join(format!("partition_{}.jsonl", partition));
            self.write_to_spill_file(&spill_path, &records)?;

            partitions.insert(
                partition,
                CacheEntry::OnDisk {
                    path: spill_path,
                    record_count: records.len(),
                },
            );
        }

        Ok(())
    }

    /// Write records to spill file
    fn write_to_spill_file(
        &self,
        path: &Path,
        records: &[serde_json::Value],
    ) -> Result<(), String> {
        let file = fs::File::create(path)
            .map_err(|e| format!("Failed to create spill file {}: {}", path.display(), e))?;
        let mut writer = BufWriter::new(file);

        for record in records {
            let line = serde_json::to_string(record)
                .map_err(|e| format!("Failed to serialize record: {}", e))?;
            writeln!(writer, "{}", line)
                .map_err(|e| format!("Failed to write to spill file: {}", e))?;
        }

        writer.flush()
            .map_err(|e| format!("Failed to flush spill file: {}", e))?;
        Ok(())
    }

    /// Append records to existing spill file
    fn append_to_spill_file(
        &self,
        path: &Path,
        records: &[serde_json::Value],
        _existing_count: usize,
    ) -> Result<(), String> {
        let file = fs::OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open spill file for append {}: {}", path.display(), e))?;
        let mut writer = BufWriter::new(file);

        for record in records {
            let line = serde_json::to_string(record)
                .map_err(|e| format!("Failed to serialize record: {}", e))?;
            writeln!(writer, "{}", line)
                .map_err(|e| format!("Failed to append to spill file: {}", e))?;
        }

        writer.flush()
            .map_err(|e| format!("Failed to flush spill file: {}", e))?;
        Ok(())
    }

    /// Load records from spill file
    fn load_from_spill_file(&self, path: &Path) -> Result<Vec<serde_json::Value>, String> {
        let file = fs::File::open(path)
            .map_err(|e| format!("Failed to open spill file {}: {}", path.display(), e))?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let record: serde_json::Value = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;
            records.push(record);
        }

        Ok(records)
    }
}

