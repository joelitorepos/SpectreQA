// src-tauri/src/storage.rs
// Acceso a disco para los archivos de cada proyecto.
// Todos los paths se derivan de projects_base/{project_id}/.
//
// Estructura de carpeta por proyecto:
//   projects/{project_id}/
//     AGENT.md          — objetivo general, editable por el usuario
//     mapas.json        — lista de snapshots DOM por fase [ [fase0_elementos], [fase1_elementos], ... ]
//     respuestas.json   — lista de respuestas de la IA por fase [ {phase, thought, status, commands}, ... ]

use std::fs;
use std::path::PathBuf;
use serde_json::Value;

use crate::session::PhaseRecord;

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

fn mapas_path(base: &PathBuf, project_id: &str) -> PathBuf {
    project_dir(base, project_id).join("mapas.json")
}

fn respuestas_path(base: &PathBuf, project_id: &str) -> PathBuf {
    project_dir(base, project_id).join("respuestas.json")
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

// ─────────────────────────────────────────────
// MAPAS.JSON — snapshots DOM por fase
// ─────────────────────────────────────────────

/// Lee todos los snapshots DOM guardados del proyecto.
/// Devuelve un Vec donde el índice es el número de fase.
/// Si el archivo no existe o está vacío, devuelve Vec vacío.
pub fn read_mapas(base: &PathBuf, project_id: &str) -> StorageResult<Vec<Value>> {
    let path = mapas_path(base, project_id);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer mapas.json: {}", e))?;
    let parsed: Vec<Value> = serde_json::from_str(&raw)
        .map_err(|e| format!("mapas.json tiene formato inválido: {}", e))?;
    Ok(parsed)
}

/// Agrega el snapshot DOM de una nueva fase al final de mapas.json.
/// `dom_elements` es el array de elementos que mandó la extensión.
pub fn append_mapa(base: &PathBuf, project_id: &str, dom_elements: Value) -> StorageResult<()> {
    let mut mapas = read_mapas(base, project_id)?;
    mapas.push(dom_elements);
    let serialized = serde_json::to_string_pretty(&mapas)
        .map_err(|e| format!("Error serializando mapas.json: {}", e))?;
    fs::write(mapas_path(base, project_id), serialized)
        .map_err(|e| format!("No se pudo escribir mapas.json: {}", e))?;
    Ok(())
}

/// Devuelve el snapshot DOM de la fase anterior (phase - 1), si existe.
/// Usado por dom_analyzer para comparar si el DOM cambió.
pub fn get_previous_mapa(base: &PathBuf, project_id: &str, current_phase: u32) -> StorageResult<Option<Value>> {
    if current_phase == 0 {
        return Ok(None);
    }
    let mapas = read_mapas(base, project_id)?;
    let prev_index = (current_phase - 1) as usize;
    Ok(mapas.get(prev_index).cloned())
}

// ─────────────────────────────────────────────
// RESPUESTAS.JSON — historial de respuestas de la IA
// ─────────────────────────────────────────────

/// Lee todas las respuestas de la IA guardadas del proyecto.
/// Devuelve Vec<PhaseRecord> ordenado por fase.
/// Si el archivo no existe o está vacío, devuelve Vec vacío.
pub fn read_respuestas(base: &PathBuf, project_id: &str) -> StorageResult<Vec<PhaseRecord>> {
    let path = respuestas_path(base, project_id);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer respuestas.json: {}", e))?;
    if raw.trim().is_empty() || raw.trim() == "[]" {
        return Ok(vec![]);
    }
    let parsed: Vec<PhaseRecord> = serde_json::from_str(&raw)
        .map_err(|e| format!("respuestas.json tiene formato inválido: {}", e))?;
    Ok(parsed)
}

/// Agrega la respuesta de la IA de una fase al final de respuestas.json.
pub fn append_respuesta(base: &PathBuf, project_id: &str, record: &PhaseRecord) -> StorageResult<()> {
    let mut respuestas = read_respuestas(base, project_id)?;
    respuestas.push(record.clone());
    let serialized = serde_json::to_string_pretty(&respuestas)
        .map_err(|e| format!("Error serializando respuestas.json: {}", e))?;
    fs::write(respuestas_path(base, project_id), serialized)
        .map_err(|e| format!("No se pudo escribir respuestas.json: {}", e))?;
    Ok(())
}

/// Devuelve la última respuesta guardada de la IA, si existe.
/// Usado para re-ejecutar comandos cuando el DOM no cambió.
pub fn get_last_respuesta(base: &PathBuf, project_id: &str) -> StorageResult<Option<PhaseRecord>> {
    let respuestas = read_respuestas(base, project_id)?;
    Ok(respuestas.into_iter().last())
}

// ─────────────────────────────────────────────
// RESET DE HISTORIAL
// ─────────────────────────────────────────────

/// Limpia mapas.json y respuestas.json del proyecto para empezar una prueba nueva.
/// Se llama al inicio de cada ejecución desde cero (no desde re-ejecución).
pub fn reset_history(base: &PathBuf, project_id: &str) -> StorageResult<()> {
    fs::write(mapas_path(base, project_id), "[]")
        .map_err(|e| format!("No se pudo resetear mapas.json: {}", e))?;
    fs::write(respuestas_path(base, project_id), "[]")
        .map_err(|e| format!("No se pudo resetear respuestas.json: {}", e))?;
    Ok(())
}

// ─────────────────────────────────────────────
// INICIALIZACIÓN DE PROYECTO
// ─────────────────────────────────────────────

/// Asegura que la carpeta del proyecto y sus archivos base existen.
/// Idempotente: si ya existen no los sobreescribe.
pub fn ensure_project_files(base: &PathBuf, project_id: &str) -> StorageResult<()> {
    let dir = project_dir(base, project_id);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo crear carpeta del proyecto: {}", e))?;

    let mapas = mapas_path(base, project_id);
    if !mapas.exists() {
        fs::write(&mapas, "[]")
            .map_err(|e| format!("No se pudo crear mapas.json: {}", e))?;
    }

    let respuestas = respuestas_path(base, project_id);
    if !respuestas.exists() {
        fs::write(&respuestas, "[]")
            .map_err(|e| format!("No se pudo crear respuestas.json: {}", e))?;
    }

    Ok(())
}