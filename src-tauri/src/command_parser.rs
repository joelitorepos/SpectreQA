// src-tauri/src/command_parser.rs

use serde::Deserialize;
use serde_json::Value;
use regex::Regex;

#[derive(Debug, Clone, Deserialize)]
pub struct RawAction {
    pub cmd: String,
    pub id: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OllamaResponse {
    pub thought: String,
    pub status: String,  // "CONTINUE" | "SUCCESS" | "FAILURE" | "ERROR_NO_CHANGE"
    pub commands: Vec<String>,
}

/**
 * Función principal unificada.
 * SIEMPRE extrae el status del JSON de respuesta.
 * Funciona tanto para Ollama (que ahora incluye status en prompt2.txt)
 * como para el servicio FAST Cloud.
 */
pub fn sanitize_response(raw: &str) -> Result<OllamaResponse, String> {
    let (thought, commands) = extract_thought_and_commands(raw)?;
    let status = extract_status(raw).unwrap_or_else(|| "CONTINUE".to_string());
    Ok(OllamaResponse {
        thought,
        status,
        commands,
    })
}

/**
 * Extrae thought y commands de la respuesta.
 */
fn extract_thought_and_commands(raw: &str) -> Result<(String, Vec<String>), String> {
    // Intentar extraer JSON primero
    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
            return parse_thought_and_commands(&parsed);
        }
    }

    // Fallback: extraer por regex
    parse_thought_and_commands_by_regex(raw)
}

/**
 * Extrae thought de un JSON.
 */
fn parse_thought_and_commands(parsed: &Value) -> Result<(String, Vec<String>), String> {
    let thought = parsed
        .get("thought")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut commands = Vec::new();

    // Intentar leer "actions" o "commands" del JSON
    if let Some(actions_array) = parsed.get("actions").and_then(|v| v.as_array()) {
        for action_value in actions_array {
            if let Some(action_str) = action_value.as_str() {
                let cleaned = clean_command(action_str);
                commands.push(cleaned);
            }
        }
    } else if let Some(commands_array) = parsed.get("commands").and_then(|v| v.as_array()) {
        for cmd_value in commands_array {
            if let Some(cmd_str) = cmd_value.as_str() {
                let cleaned = clean_command(cmd_str);
                commands.push(cleaned);
            }
        }
    }

    let valid_commands = validate_commands(&commands);
    Ok((thought, valid_commands))
}

/**
 * Extrae thought por regex (fallback).
 */
fn parse_thought_and_commands_by_regex(raw: &str) -> Result<(String, Vec<String>), String> {
    let mut thought = String::new();
    let mut commands = Vec::new();

    // Extraer thought
    let thought_re = Regex::new(r#"(?i)"thought"\s*:\s*"([^"]*)"#).unwrap();
    if let Some(cap) = thought_re.captures(raw) {
        thought = cap[1].to_string();
    }

    // Extraer comandos con regex
    let cmd_re = Regex::new(r#"@(Click|Write|Wait|WriteRandom|WriteRandomNum)\s+([\w\-]+)(?:\s+"([^"]*)")?"#).unwrap();
    for cap in cmd_re.captures_iter(raw) {
        let cmd_type = &cap[1];
        let id = &cap[2];
        let text = cap.get(3).map(|m| m.as_str());

        let full_cmd = match cmd_type {
            "Click" => format!("@Click {}", id),
            "Wait" => format!("@Wait {}", id),
            "Write" => {
                if let Some(t) = text {
                    format!("@Write {} \"{}\"", id, t)
                } else {
                    format!("@Write {} \"\"", id)
                }
            }
            "WriteRandom" => format!("@WriteRandom {} {}", id, text.unwrap_or("10")),
            "WriteRandomNum" => format!("@WriteRandomNum {} {}", id, text.unwrap_or("6")),
            _ => continue,
        };
        commands.push(full_cmd);
    }

    // Si no encontramos comandos, probar formato legacy
    if commands.is_empty() {
        let legacy_re = Regex::new(r#"@(Click|Write|Wait|WriteRandom|WriteRandomNum)\s+([\w\-]+)(?:\s+([\w\-]+))?"#).unwrap();
        for cap in legacy_re.captures_iter(raw) {
            let cmd_type = &cap[1];
            let id = &cap[2];
            let param = cap.get(3).map(|m| m.as_str());

            let full_cmd = match cmd_type {
                "Click" => format!("@Click {}", id),
                "Wait" => format!("@Wait {}", param.unwrap_or("500")),
                "Write" => format!("@Write {} \"{}\"", id, param.unwrap_or("")),
                "WriteRandom" => format!("@WriteRandom {} {}", id, param.unwrap_or("10")),
                "WriteRandomNum" => format!("@WriteRandomNum {} {}", id, param.unwrap_or("6")),
                _ => continue,
            };
            commands.push(full_cmd);
        }
    }

    let valid_commands = validate_commands(&commands);
    Ok((thought, valid_commands))
}

/**
 * Extrae el status del JSON.
 */
fn extract_status(raw: &str) -> Option<String> {
    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
            return parsed
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
    }
    // Fallback: regex para status
    let status_re = Regex::new(r#"(?i)"status"\s*:\s*"(CONTINUE|SUCCESS|FAILURE|ERROR_NO_CHANGE)""#).unwrap();
    if let Some(cap) = status_re.captures(raw) {
        return Some(cap[1].to_string());
    }
    None
}

/**
 * Limpia un comando de comillas extrañas y espacios.
 */
fn clean_command(cmd: &str) -> String {
    let mut cleaned = cmd.trim().to_string();
    cleaned = cleaned.replace('\'', "");
    if cleaned.contains(" @Click id ") || cleaned.contains("@Click id ") {
        cleaned = cleaned.replace("@Click id ", "@Click ");
    }
    if cleaned.contains(" @Write id ") || cleaned.contains("@Write id ") {
        cleaned = cleaned.replace("@Write id ", "@Write ");
    }
    cleaned
}

/**
 * Extrae el bloque JSON ignorando markdown o texto adyacente.
 */
fn extract_json_block(raw: &str) -> Option<String> {
    if let Some(start_idx) = raw.find('{') {
        if let Some(end_idx) = raw.rfind('}') {
            if end_idx > start_idx {
                return Some(raw[start_idx..=end_idx].to_string());
            }
        }
    }
    None
}

/**
 * Filtra los comandos para asegurar que cumplan con la sintaxis mínima requerida.
 */
pub fn validate_commands(commands: &[String]) -> Vec<String> {
    commands
        .iter()
        .filter(|c| {
            c.starts_with("@Click ")
            || c.starts_with("@Write ")
            || c.starts_with("@Wait ")
            || c.starts_with("@WriteRandom ")
            || c.starts_with("@WriteRandomNum ")
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_with_status() {
        let response = r#"{
            "thought": "Login successful",
            "status": "SUCCESS",
            "actions": ["@Click button-1"]
        }"#;
        let result = sanitize_response(response).unwrap();
        assert_eq!(result.status, "SUCCESS");
        assert_eq!(result.thought, "Login successful");
    }

    #[test]
    fn test_response_without_status_defaults_continue() {
        let response = r#"{
            "thought": "Still logging in",
            "actions": ["@Click button-1"]
        }"#;
        let result = sanitize_response(response).unwrap();
        assert_eq!(result.status, "CONTINUE");
    }

    #[test]
    fn test_response_with_error_status() {
        let response = r#"{
            "thought": "Login failed",
            "status": "FAILURE",
            "commands": []
        }"#;
        let result = sanitize_response(response).unwrap();
        assert_eq!(result.status, "FAILURE");
    }

    #[test]
    fn test_response_with_commands_array() {
        let response = r#"{
            "thought": "Fill form",
            "status": "CONTINUE",
            "commands": ["@Write input-1 \"test\"", "@Click button-1"]
        }"#;
        let result = sanitize_response(response).unwrap();
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0], "@Write input-1 \"test\"");
        assert_eq!(result.commands[1], "@Click button-1");
    }

    #[test]
    fn test_response_with_actions_array() {
        let response = r#"{
            "thought": "Fill form",
            "status": "CONTINUE",
            "actions": ["@Write input-1 \"test\"", "@Click button-1"]
        }"#;
        let result = sanitize_response(response).unwrap();
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0], "@Write input-1 \"test\"");
        assert_eq!(result.commands[1], "@Click button-1");
    }
}