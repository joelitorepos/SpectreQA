// src-tauri/src/prompt_builder.rs

use serde_json::Value;
use crate::session::PhaseRecord;

/**
 * Construye el prompt completo para Ollama sustituyendo los placeholders
 * del template en prompt.txt:
 *   {AGENT.md}              — objetivo general del proyecto
 *   {HISTORIAL_DE_FASES}    — fases anteriores formateadas
 *   {NUMERO_FASE}           — número de fase actual
 *   {JSON_SANITIZADO_DEL_DOM} — snapshot DOM compacto (con inputs ofuscados)
 *   {URL_ANTERIOR}          — URL previa conocida (o "(sin dato)")
 *   {URL_ACTUAL}            — URL actual de la página
 */
pub fn build_prompt(
    agent_md: &str,
    history: &[PhaseRecord],
    phase_num: u32,
    dom: &Value,
    previous_url: Option<&str>,
    current_url: Option<&str>,
) -> String {
    let template = include_str!("../resources/prompt.txt");

    let history_str = format_history(history);
    let dom_str = serialize_dom(dom); // Ahora ofusca los inputs internamente
    let previous_url_str = previous_url.unwrap_or("(sin dato)");
    let current_url_str = current_url.unwrap_or("(sin dato)");

    let prompt = template
        .replace("{AGENT.md}", agent_md.trim())
        .replace("{HISTORIAL_DE_FASES}", &history_str)
        .replace("{NUMERO_FASE}", &phase_num.to_string())
        .replace("{JSON_SANITIZADO_DEL_DOM}", &dom_str)
        .replace("{URL_ANTERIOR}", previous_url_str)
        .replace("{URL_ACTUAL}", current_url_str);

    let prompt_preview = prompt.chars().take(100).collect::<String>();
    println!("[SpectreQA] Prompt generado para fase {}:\n{}\n", phase_num, prompt_preview);

    println!("[SpectreQA] Prompt completo:\n{}\n", prompt);
    prompt
}

/**
 * Formatea el historial de fases anteriores en texto legible para la IA.
 * Si no hay historial, devuelve un string indicando que es la primera fase.
 */
pub fn format_history(history: &[PhaseRecord]) -> String {
    if history.is_empty() {
        return "# No previous phases executed yet.".to_string();
    }

    history
        .iter()
        .map(|record| {
            // Dar formato de strings de Python a cada comando de la lista
            let commands_str = record
                .commands
                .iter()
                .map(|c| format!("\"{}\"", c.replace('"', "\\\""))) // Escapar comillas internas si las hay
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "    {{\n        \"phase\": {},\n        \"thought\": \"{}\",\n        \"status\": \"{}\",\n        \"commands\": [{}]\n    }},",
                record.phase,
                record.thought.replace('"', "\\\""), // Escapar comillas en el razonamiento
                record.status,
                commands_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/**
 * Serializa el snapshot DOM a JSON compacto, pero antes ofusca el contenido
 * de los inputs para proteger la privacidad.
 */
fn serialize_dom(dom: &Value) -> String {
    // Clonamos para no mutar el original (que podría usarse en otro lado)
    let mut dom_clone = dom.clone();
    // Ofuscamos los inputs en la copia
    obfuscate_dom_inputs(&mut dom_clone);
    // Serializamos a JSON compacto
    dom_clone.to_string()
}

/**
 * Función recursiva que recorre el árbol JSON y reemplaza el contenido
 * de los nodos INPUT por indicadores genéricos.
 *
 * Criterios de detección:
 *   - Objeto con campo "tagName" igual a "INPUT" (insensible a mayúsculas)
 *   - O bien, si el campo "id" contiene la palabra "input" (para capturar variantes)
 *
 * Se ignoran los LABEL y cualquier otro elemento.
 */
pub fn obfuscate_dom_inputs(dom: &mut Value) {
    match dom {
        Value::Object(map) => {
            // Primero, comprobamos si este objeto es un INPUT
            let is_input = if let Some(tag) = map.get("tagName").and_then(|v| v.as_str()) {
                tag.eq_ignore_ascii_case("INPUT")
            } else if let Some(id) = map.get("id").and_then(|v| v.as_str()) {
                // Si no tiene tagName, pero el id contiene "input", lo tratamos como input
                id.to_lowercase().contains("input")
            } else {
                false
            };

            // Si es input, ofuscamos su campo "content" (o "value" si existiera)
            if is_input {
                if let Some(content) = map.get_mut("content") {
                    if let Some(s) = content.as_str() {
                        let new_content = if s.is_empty() {
                            "[empty]".to_string()
                        } else {
                            let len = s.chars().count(); // caracteres, no bytes
                            format!("[filled:{}chars]", len)
                        };
                        *content = Value::String(new_content);
                    }
                }
                // Si el campo se llama "value" en lugar de "content", también lo manejamos
                if let Some(value) = map.get_mut("value") {
                    if let Some(s) = value.as_str() {
                        let new_value = if s.is_empty() {
                            "[empty]".to_string()
                        } else {
                            let len = s.chars().count();
                            format!("[filled:{}chars]", len)
                        };
                        *value = Value::String(new_value);
                    }
                }
                // No procesamos más los hijos, porque un input no suele tener hijos
                return;
            }

            // Si no es input, procesamos recursivamente cada valor del objeto
            for (_, value) in map.iter_mut() {
                obfuscate_dom_inputs(value);
            }
        }
        Value::Array(arr) => {
            // Procesamos cada elemento del array
            for item in arr.iter_mut() {
                obfuscate_dom_inputs(item);
            }
        }
        _ => {} // Otros tipos (string, número, etc.) no se procesan
    }
}