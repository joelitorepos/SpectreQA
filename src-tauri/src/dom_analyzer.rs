// src-tauri/src/dom_analyzer.rs

use serde_json::Value;

/**
 * Compara dos snapshots DOM y determina si hubo un cambio significativo.
 * Ignora coordenadas (x, y, width, height) porque cambian con el scroll
 * y no indican un cambio real en la UI.
 * Devuelve true si el DOM cambió, false si es idéntico.
 */
pub fn has_dom_changed(previous: &Value, current: &Value) -> bool {
    let prev_fingerprint = extract_fingerprint(previous);
    let curr_fingerprint = extract_fingerprint(current);
    prev_fingerprint != curr_fingerprint
}

/**
 * Extrae un fingerprint del snapshot DOM ignorando coordenadas.
 * El fingerprint es una lista de strings con los campos relevantes
 * de cada elemento: id, content, disabled.
 * El orden importa — si un elemento aparece o desaparece, el fingerprint cambia.
 */
fn extract_fingerprint(dom: &Value) -> Vec<String> {
    let elements = match dom.as_array() {
        Some(arr) => arr,
        None => return vec![],
    };

    elements
        .iter()
        .map(|el| {
            let id = el.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let content = el.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let disabled = el.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false);
            format!("{}|{}|{}", id, content, disabled)
        })
        .collect()
}