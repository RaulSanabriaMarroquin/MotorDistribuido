//! Caché con derrame a disco para procesamiento por lotes
//! Implementa caché en memoria con derrame automático a disco cuando se excede el umbral

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Umbral configurable (según especificación sección 4.3)
// Puede establecerse mediante variable de entorno CACHE_THRESHOLD_MB, por defecto 100MB
fn get_cache_threshold_mb() -> usize {
    std::env::var("CACHE_THRESHOLD_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

const CACHE_DIR: &str = "cache";

/// Entrada de caché que puede estar en memoria o en disco
#[derive(Debug, Clone)]
pub enum CacheEntry {
    InMemory(Vec<i64>),
    OnDisk(PathBuf),
}

/// Administrador de caché con derrame a disco
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
    memory_usage_mb: usize,
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new() -> Self {
        let cache_dir = PathBuf::from(CACHE_DIR);
        Self {
            entries: HashMap::new(),
            memory_usage_mb: 0,
            cache_dir,
        }
    }

    /// Almacenar datos en caché (derramar a disco si es necesario)
    pub async fn store(&mut self, key: &str, data: Vec<i64>) -> Result<(), String> {
        let threshold_mb = get_cache_threshold_mb();
        let size_mb = (data.len() * std::mem::size_of::<i64>()) / (1024 * 1024);
        
        // Verificar si necesitamos derramar a disco
        if self.memory_usage_mb + size_mb > threshold_mb {
            // Derramar entradas existentes a disco
            self.spill_to_disk().await?;
        }

        // Si aún está sobre el umbral, almacenar directamente en disco
        if size_mb > threshold_mb / 2 {
            let disk_path = self.store_to_disk(key, &data).await?;
            self.entries.insert(key.to_string(), CacheEntry::OnDisk(disk_path));
        } else {
            self.memory_usage_mb += size_mb;
            self.entries.insert(key.to_string(), CacheEntry::InMemory(data));
        }

        Ok(())
    }

    /// Recuperar datos del caché
    pub async fn retrieve(&self, key: &str) -> Result<Option<Vec<i64>>, String> {
        match self.entries.get(key) {
            Some(CacheEntry::InMemory(data)) => Ok(Some(data.clone())),
            Some(CacheEntry::OnDisk(path)) => {
                self.load_from_disk(path).await
            }
            None => Ok(None),
        }
    }

    /// Almacenar datos en disco
    async fn store_to_disk(&self, key: &str, data: &[i64]) -> Result<PathBuf, String> {
        // Crear directorio de caché si no existe
        fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| format!("Error al crear directorio de caché: {}", e))?;

        let file_path = self.cache_dir.join(format!("{}.cache", key));
        let mut file = fs::File::create(&file_path)
            .await
            .map_err(|e| format!("Error al crear archivo de caché: {}", e))?;

        // Escribir datos como arreglo JSON
        let json = serde_json::to_string(data)
            .map_err(|e| format!("Error al serializar datos de caché: {}", e))?;
        
        file.write_all(json.as_bytes())
            .await
            .map_err(|e| format!("Error al escribir archivo de caché: {}", e))?;

        file.flush()
            .await
            .map_err(|e| format!("Error al hacer flush del archivo de caché: {}", e))?;

        Ok(file_path)
    }

    /// Cargar datos desde disco
    async fn load_from_disk(&self, path: &Path) -> Result<Option<Vec<i64>>, String> {
        let mut file = fs::File::open(path)
            .await
            .map_err(|e| format!("Error al abrir archivo de caché: {}", e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| format!("Error al leer archivo de caché: {}", e))?;

        let data: Vec<i64> = serde_json::from_str(&contents)
            .map_err(|e| format!("Error al deserializar datos de caché: {}", e))?;

        Ok(Some(data))
    }

    /// Derramar entradas existentes en memoria a disco
    async fn spill_to_disk(&mut self) -> Result<(), String> {
        let mut keys_to_spill = Vec::new();
        
        // Encontrar entradas para derramar (más antiguas primero, simplificado: derramar todas)
        for (key, entry) in &self.entries {
            if matches!(entry, CacheEntry::InMemory(_)) {
                keys_to_spill.push(key.clone());
            }
        }

        // Derramar entradas a disco
        for key in keys_to_spill {
            if let Some(CacheEntry::InMemory(data)) = self.entries.remove(&key) {
                let size_mb = (data.len() * std::mem::size_of::<i64>()) / (1024 * 1024);
                let disk_path = self.store_to_disk(&key, &data).await?;
                self.entries.insert(key, CacheEntry::OnDisk(disk_path));
                self.memory_usage_mb = self.memory_usage_mb.saturating_sub(size_mb);
            }
        }

        Ok(())
    }

    /// Limpiar caché
    pub async fn clear(&mut self) -> Result<(), String> {
        // Eliminar archivos de disco
        for entry in self.entries.values() {
            if let CacheEntry::OnDisk(path) = entry {
                let _ = fs::remove_file(path).await;
            }
        }
        
        self.entries.clear();
        self.memory_usage_mb = 0;
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

