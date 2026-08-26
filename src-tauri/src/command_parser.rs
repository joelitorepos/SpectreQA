// src-tauri/src/command_parser.rs

use regex::Regex;

// Derivamos Serialize para poder transformar este objeto directo a JSON hacia el WebSocket
#[derive(Debug, Clone, serde::Serialize)] 
pub struct OllamaResponse {
    pub thought: String,
    pub status: String,
    pub commands: Vec<String>,
    pub actions: Vec<String>, // Duplicado para compatibilidad estricta con la extensión
}

pub fn sanitize_response(raw: &str) -> Result<OllamaResponse, String> {
    // CAMINO FELIZ: si raw ya es JSON válido con la forma esperada (siempre
    // el caso en modo cloud — el backend hace su propio JSON.parse antes de
    // devolver), tomamos el array "commands" directo. Cero regex de por
    // medio, así que una palabra como "click" dentro del "thought" jamás
    // puede confundirse con un comando.
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(commands_arr) = parsed.get("commands").and_then(|v| v.as_array()) {
            let thought = parsed.get("thought").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let commands: Vec<String> = commands_arr.iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect();

            println!("[SpectreQA - Parser] JSON válido — comandos tomados directo del array, sin regex.");
            println!("  Thought: {}", thought);
            println!("  Commands/Actions: {:?}", commands);

            return Ok(OllamaResponse {
                thought,
                status: "CONTINUE".to_string(),
                commands: commands.clone(),
                actions: commands,
            });
        }
    }

    // CAMINO DE TOLERANCIA A RUIDO: modo local, donde el modelo a veces
    // envuelve la respuesta en markdown o texto extra y raw no es JSON
    // válido. Acá SÍ usamos regex, pero acotado solo al contenido del array
    // "commands" — nunca al "thought", que es texto libre en inglés/español
    // y puede contener palabras como "click" o "write" sin ser un comando.
    let search_area = extract_commands_section(raw);

    // 1. Extraer el "thought" (esto sí puede leerse del raw completo, es
    // solo texto descriptivo, no hay riesgo de falsos positivos).
    let thought_re = Regex::new(r#"(?is)"thought"\s*:\s*"(.*?)""#).unwrap();
    let thought = match thought_re.captures(raw) {
        Some(caps) => caps.get(1).map_or("", |m| m.as_str())
            .replace(r#"\"#, "") 
            .to_string(),
        None => String::new(),
    };

    // 2. Extraer y ensamblar los comandos secuencialmente — SOLO dentro de search_area.
    let mut valid_commands = Vec::new();

    let command_block_re = Regex::new(r#"(?i)@(Click|Write|WriteRandom|WriteRandomNum)\b([^@\n\]\}]*)"#).unwrap();
    let id_re = Regex::new(r"([a-zA-Z0-9_\-\:]+)").unwrap();
    let num_re = Regex::new(r"(\d+)").unwrap();
    let string_re = Regex::new(r#""(.*?)""#).unwrap();

    for cap in command_block_re.captures_iter(search_area) {
        let cmd_type = cap.get(1).unwrap().as_str().to_lowercase();
        let params_raw = cap.get(2).unwrap().as_str();

        match cmd_type.as_str() {
            "click" => {
                if let Some(id_match) = id_re.captures(params_raw) {
                    let clean_id = id_match.get(1).unwrap().as_str();
                    valid_commands.push(format!("@Click {}", clean_id));
                }
            }
            "write" => {
                if let Some(id_match) = id_re.captures(params_raw) {
                    let clean_id = id_match.get(1).unwrap().as_str();
                    
                    let text = if let Some(str_match) = string_re.captures(params_raw) {
                        str_match.get(1).unwrap().as_str()
                            .replace(r#"\"#, "")
                            .trim_matches('"')
                            .to_string()
                    } else {
                        String::new()
                    };
                    
                    valid_commands.push(format!("@Write {} \"{}\"", clean_id, text));
                }
            }
            "writerandom" => {
                if let Some(id_match) = id_re.captures(params_raw) {
                    let clean_id = id_match.get(1).unwrap().as_str();
                    let len = num_re.captures(params_raw).map_or("10", |n| n.get(1).unwrap().as_str());
                    valid_commands.push(format!("@WriteRandom {} {}", clean_id, len));
                }
            }
            "writerandomnum" => {
                if let Some(id_match) = id_re.captures(params_raw) {
                    let clean_id = id_match.get(1).unwrap().as_str();
                    let len = num_re.captures(params_raw).map_or("6", |n| n.get(1).unwrap().as_str());
                    valid_commands.push(format!("@WriteRandomNum {} {}", clean_id, len));
                }
            }
            _ => {}
        }
    }

    println!("[SpectreQA - Parser] Reconstrucción vía regex (raw no era JSON válido):");
    println!("  Thought: {}", thought);
    println!("  Commands/Actions: {:?}", valid_commands);

    Ok(OllamaResponse {
        thought,
        status: "CONTINUE".to_string(), 
        commands: valid_commands.clone(),
        actions: valid_commands, 
    })
}

/**
 * Acota el texto de búsqueda del regex al contenido del array "commands",
 * para que nunca escanee el "thought" (texto libre) buscando comandos.
 * Si no encuentra la estructura esperada, cae de vuelta al raw completo
 * (mismo comportamiento que antes de este fix) en vez de fallar duro.
 */
fn extract_commands_section(raw: &str) -> &str {
    let Some(key_pos) = raw.find("\"commands\"") else { return raw; };
    let after_key = &raw[key_pos..];
    let Some(bracket_offset) = after_key.find('[') else { return raw; };
    let start = key_pos + bracket_offset;
    let Some(close_offset) = raw[start..].find(']') else { return raw; };
    let end = start + close_offset + 1;
    &raw[start..end]
}