// src-tauri/src/storage.rs
// Acceso a disco para los archivos de cada proyecto.
// Todos los paths se derivan de projects_base/{project_id}/.
//
// Estructura de carpeta por proyecto:
//   projects/{project_id}/
//     AGENT.md          — objetivo general, editable por el usuario
//     logs/             — un archivo JSON por cada prueba que terminó
//       log_{started_at}_{outcome}.json

use std::fs;
use std::path::PathBuf;

use crate::session::{TestLogEntry, TestLogSummary, TestOutcome};

// TIPOS DE ERROR

pub type StorageResult<T> = Result<T, String>;

// HELPERS DE PATH

fn project_dir(base: &PathBuf, project_id: &str) -> PathBuf {
    base.join(project_id)
}

fn agent_md_path(base: &PathBuf, project_id: &str) -> PathBuf {
    project_dir(base, project_id).join("AGENT.md")
}

fn logs_dir(base: &PathBuf, project_id: &str) -> PathBuf {
    project_dir(base, project_id).join("logs")
}

fn log_id_for(started_at: u64, outcome: TestOutcome) -> String {
    format!("log_{}_{}", started_at, outcome.as_filename_part())
}

fn log_path(base: &PathBuf, project_id: &str, log_id: &str) -> PathBuf {
    logs_dir(base, project_id).join(format!("{}.json", log_id))
}

/// Evita que un log_id con "/" o ".." se escape de la carpeta de logs.
fn is_safe_log_id(log_id: &str) -> bool {
    !log_id.is_empty()
        && !log_id.contains('/')
        && !log_id.contains('\\')
        && !log_id.contains("..")
}

// AGENT.MD

/// Lee el contenido del AGENT.md del proyecto.
#[allow(dead_code)]
pub fn read_agent_md(base: &PathBuf, project_id: &str) -> StorageResult<String> {
    let path = agent_md_path(base, project_id);
    fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer AGENT.md para '{}': {}", project_id, e))
}

// LOGS DE PRUEBA

/// Guarda un log de prueba terminada. Devuelve el log_id generado.
pub fn save_test_log(base: &PathBuf, entry: &TestLogEntry) -> StorageResult<String> {
    let dir = logs_dir(base, &entry.project_id);
    fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear carpeta de logs: {}", e))?;

    let log_id = log_id_for(entry.started_at, entry.outcome);
    let path = dir.join(format!("{}.json", log_id));

    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| format!("No se pudo serializar el log: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("No se pudo guardar el log: {}", e))?;

    println!("[SpectreQA] Log de prueba guardado: {}", log_id);
    Ok(log_id)
}

/// Lista los logs de un proyecto sin abrir cada archivo: toda la info sale
/// del nombre (log_{started_at}_{outcome}.json). Se ordenan del más reciente
/// al más antiguo.
pub fn list_test_logs(base: &PathBuf, project_id: &str) -> StorageResult<Vec<TestLogSummary>> {
    let dir = logs_dir(base, project_id);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&dir).map_err(|e| format!("No se pudo leer carpeta de logs: {}", e))?;

    let mut summaries: Vec<TestLogSummary> = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = match file_name.to_str() {
            Some(n) => n,
            None => continue,
        };

        let log_id = match name.strip_suffix(".json") {
            Some(stripped) => stripped,
            None => continue,
        };

        if let Some(summary) = parse_log_id(log_id) {
            summaries.push(summary);
        }
    }

    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(summaries)
}

/// Parsea "log_{started_at}_{outcome}" -> TestLogSummary. None si el nombre
/// no tiene el formato esperado (por ejemplo, un archivo dejado a mano).
fn parse_log_id(log_id: &str) -> Option<TestLogSummary> {
    let rest = log_id.strip_prefix("log_")?;
    let (started_at_str, outcome_str) = rest.split_once('_')?;
    let started_at: u64 = started_at_str.parse().ok()?;
    let outcome = match outcome_str {
        "success" => TestOutcome::Success,
        "error" => TestOutcome::Error,
        "terminated" => TestOutcome::Terminated,
        _ => return None,
    };
    Some(TestLogSummary {
        log_id: log_id.to_string(),
        outcome,
        started_at,
    })
}

/// Lee el contenido completo de un log (fases con pensamiento + comandos).
pub fn read_test_log(base: &PathBuf, project_id: &str, log_id: &str) -> StorageResult<TestLogEntry> {
    if !is_safe_log_id(log_id) {
        return Err("log_id inválido".to_string());
    }
    let path = log_path(base, project_id, log_id);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer el log '{}': {}", log_id, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Log corrupto '{}': {}", log_id, e))
}

/// Elimina un log de prueba específico.
pub fn delete_test_log(base: &PathBuf, project_id: &str, log_id: &str) -> StorageResult<()> {
    if !is_safe_log_id(log_id) {
        return Err("log_id inválido".to_string());
    }
    let path = log_path(base, project_id, log_id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("No se pudo eliminar el log '{}': {}", log_id, e))?;
    }
    Ok(())
}