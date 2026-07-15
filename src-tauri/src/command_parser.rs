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
    // 1. Extraer el "thought"
    let thought_re = Regex::new(r#"(?is)"thought"\s*:\s*"(.*?)""#).unwrap();
    let thought = match thought_re.captures(raw) {
        Some(caps) => caps.get(1).map_or("", |m| m.as_str())
            .replace(r#"\"#, "") 
            .to_string(),
        None => String::new(),
    };

    // 2. Extraer y ensamblar los comandos secuencialmente
    let mut valid_commands = Vec::new();

    let command_block_re = Regex::new(r#"(?i)(?:@)?(Click|Write|WriteRandom|WriteRandomNum)\b([^@\n\]\}]*)"#).unwrap();
    let id_re = Regex::new(r"([a-zA-Z0-9_\-\:]+)").unwrap();
    let num_re = Regex::new(r"(\d+)").unwrap();
    let string_re = Regex::new(r#""(.*?)""#).unwrap();

    for cap in command_block_re.captures_iter(raw) {
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
            // ✅ Eliminado el brazo "wait"
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

    println!("[SpectreQA - Parser] Reconstrucción Exitosa:");
    println!("  Thought: {}", thought);
    println!("  Commands/Actions: {:?}", valid_commands);

    Ok(OllamaResponse {
        thought,
        status: "CONTINUE".to_string(), 
        commands: valid_commands.clone(),
        actions: valid_commands, 
    })
}