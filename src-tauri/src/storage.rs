// src-tauri/src/storage.rs
// Acceso a disco para los archivos de cada proyecto.
// Todos los paths se derivan de projects_base/{project_id}/.
//
// Estructura de carpeta por proyecto:
//   projects/{project_id}/
//     AGENT.md          — objetivo general, editable por el usuario

use std::fs;
use std::path::PathBuf;

// ─────────────────────────────────────────────
// TIPOS DE ERROR
// ─────────────────────────────────────────────

pub type StorageResult<T> = Result<T, String>;

// ─────────────────────────────────────────────
// HELPERS DE PATH
// ─────────────────────────────────────────────

fn project_dir(base: &PathBuf, project_id: &str) -> PathBuf {
    base.join(project_id)
}

fn agent_md_path(base: &PathBuf, project_id: &str) -> PathBuf {
    project_dir(base, project_id).join("AGENT.md")
}

// ─────────────────────────────────────────────
// AGENT.MD
// ─────────────────────────────────────────────

/// Lee el contenido del AGENT.md del proyecto.
#[allow(dead_code)]
pub fn read_agent_md(base: &PathBuf, project_id: &str) -> StorageResult<String> {
    let path = agent_md_path(base, project_id);
    fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer AGENT.md para '{}': {}", project_id, e))
}