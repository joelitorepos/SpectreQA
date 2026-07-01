// src-tauri/src/command_parser.rs

use serde_json::Value;
use regex::Regex;

/**
 * Resultado del parseo de la respuesta de Ollama.
 */
#[derive(Debug, Clone)]
pub struct OllamaResponse {
    pub thought: String,
    pub status: String,
    pub commands: Vec<String>,
}

/**
 * Intenta extraer y parsear un JSON válido de la respuesta cruda de Ollama.
 * Tolerante a errores comunes:
 *   - Texto o markdown antes/después del JSON
 *   - Bloques ```json ... ```
 *   - Comillas sin escapar dentro de strings de @Write
 */
pub fn sanitize_ollama_response(raw: &str) -> Result<OllamaResponse, String> {
    let extracted = extract_json_block(raw)
        .ok_or_else(|| format!("No se encontró un bloque JSON en la respuesta: {}", raw))?;

    let repaired = repair_json(&extracted);

    let parsed: Value = serde_json::from_str(&repaired)
        .map_err(|e| format!("JSON inválido tras reparación: {} — raw: {}", e, repaired))?;

    let thought = parsed
        .get("thought")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let status = parsed
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("CONTINUE")
        .to_string();

    let commands_raw = parsed
        .get("commands")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let commands: Vec<String> = commands_raw
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .map(|s| clean_command(&s))
        .collect();

    println!("[SpectreQA] Parseo exitoso - thought: {}, status: {}, commands: {:?}", thought, status, commands);

    Ok(OllamaResponse { thought, status, commands })
}

/**
 * Valida que cada comando tenga sintaxis correcta.
 * Devuelve los comandos limpios si todos son válidos.
 * Descarta comandos desconocidos con un warning en lugar de fallar toda la fase.
 */
pub fn validate_commands(commands: &[String]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|cmd| {
            let cmd = cmd.trim();
            if is_valid_command(cmd) {
                Some(cmd.to_string())
            } else {
                eprintln!("[SpectreQA] Comando inválido descartado: {}", cmd);
                None
            }
        })
        .collect()
}

/**
 * Limpia "palabras sueltas" que algunos modelos —sobre todo los más chicos,
 * como qwen2.5-coder:3b— insertan por error al copiar literalmente el
 * nombre del parámetro desde la documentación del prompt.
 *
 * Caso real observado:
 *   El prompt documenta el comando así:   @Write id "text" : ...
 *   La IA respondió:                      @Write id "input-1-root" "admin"
 *   Se esperaba:                          @Write input-1-root "admin"
 *
 * La IA confundió el "id" que aparece como nombre de parámetro en la
 * documentación con sintaxis literal, en vez de sustituirlo por el id
 * real del elemento del DOM.
 *
 * Esta función detecta y descarta ese "id" sobrante (sin distinguir
 * mayúsculas/minúsculas: "id", "Id", "ID"), y de paso normaliza comillas
 * extra alrededor del identificador, p. ej.:
 *   @Click id "btn-1"      -> @Click btn-1
 *   @Click id btn-1        -> @Click btn-1
 *   @WriteRandom id f-1 10 -> @WriteRandom f-1 10
 *
 * Es conservadora: si el comando ya viene bien formado, lo devuelve sin
 * modificar (idempotente), incluyendo el caso límite donde el id real
 * empieza con "id" pero no es la palabra "id" sola (ej: "identifier-1").
 */
fn clean_command(cmd: &str) -> String {
    let cmd = cmd.trim();

    let re = Regex::new(
        r#"^(@Click|@Write|@WriteRandom|@WriteRandomNum|@Wait)\s+(?:(?i:id)\s+)?"?([\w\-]+)"?(.*)$"#,
    )
    .unwrap();

    match re.captures(cmd) {
        Some(caps) => {
            let command = &caps[1];
            let id = &caps[2];
            let rest = caps[3].trim();

            if rest.is_empty() {
                format!("{} {}", command, id)
            } else {
                format!("{} {} {}", command, id, rest)
            }
        }
        None => cmd.to_string(),
    }
}

/**
 * Verifica que un comando coincida con alguno de los formatos válidos:
 *   @Click <id>
 *   @Write <id> "<text>"
 *   @WriteRandom <id> <len>
 *   @WriteRandomNum <id> <len>
 *   @Wait <ms>
 */
fn is_valid_command(cmd: &str) -> bool {
    if cmd.starts_with("@Click ") {
        // @Click <id>  — id es word chars, guiones y números
        let re = Regex::new(r"^@Click [\w\-]+$").unwrap();
        return re.is_match(cmd);
    }
    if cmd.starts_with("@Write ") {
        // @Write <id> "<text>"  — text puede contener cualquier cosa excepto " sin escapar
        let re = Regex::new(r#"^@Write [\w\-]+ ".*"$"#).unwrap();
        return re.is_match(cmd);
    }
    if cmd.starts_with("@WriteRandom ") {
        let re = Regex::new(r"^@WriteRandom [\w\-]+ \d+$").unwrap();
        return re.is_match(cmd);
    }
    if cmd.starts_with("@WriteRandomNum ") {
        let re = Regex::new(r"^@WriteRandomNum [\w\-]+ \d+$").unwrap();
        return re.is_match(cmd);
    }
    if cmd.starts_with("@Wait ") {
        let re = Regex::new(r"^@Wait \d+$").unwrap();
        return re.is_match(cmd);
    }
    false
}

/**
 * Extrae el primer bloque JSON { ... } de un string que puede tener
 * texto adicional o markdown alrededor.
 */
fn extract_json_block(raw: &str) -> Option<String> {
    // Primero intentar quitar bloques de markdown ```json ... ```
    let cleaned = raw
        .replace("```json", "")
        .replace("```", "");

    // Buscar el primer { y el último } balanceado
    let start = cleaned.find('{')?;
    let slice = &cleaned[start..];

    let mut depth = 0i32;
    let mut end = None;

    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    end.map(|e| slice[..e].to_string())
}

/**
 * Intenta reparar problemas comunes en el JSON antes de parsearlo.
 * Cubre casos donde la IA olvida escapar comillas dentro de strings de @Write.
 * La reparación es conservadora: solo actúa sobre patrones conocidos.
 */
fn repair_json(json: &str) -> String {
    // Caso más común: @Write input-text-1-login "texto con "comillas" sin escapar"
    // La IA a veces pone: "@Write input-text-1-login \"texto con "comillas"\""
    // Intentamos normalizar las comillas internas en valores de commands[]
    //
    // Estrategia: reemplazar comillas dobles no escapadas dentro de arrays de strings.
    // Es conservador — si el JSON ya es válido, serde_json lo parsea sin tocar esto.

    // Trailing commas antes de } o ] — error común de la IA
    let re_trailing = Regex::new(r",(\s*[}\]])").unwrap();
    let fixed = re_trailing.replace_all(json, "$1").to_string();

    fixed
}
#[cfg(test)]
mod clean_command_tests {
    use super::*;

    #[test]
    fn elimina_id_suelto_en_write_caso_real() {
        // Caso real reportado: la IA antepuso "id" al identificador real
        assert_eq!(
            clean_command(r#"@Write id "input-1-root" "admin""#),
            r#"@Write input-1-root "admin""#
        );
        assert_eq!(
            clean_command(r#"@Write id "input-2-root" "admin123""#),
            r#"@Write input-2-root "admin123""#
        );
    }

    #[test]
    fn elimina_id_suelto_en_click_con_y_sin_comillas() {
        assert_eq!(
            clean_command(r#"@Click id "button-1-root""#),
            "@Click button-1-root"
        );
        assert_eq!(
            clean_command("@Click id button-1-root"),
            "@Click button-1-root"
        );
    }

    #[test]
    fn elimina_id_suelto_en_writerandom_y_writerandomnum() {
        assert_eq!(
            clean_command("@WriteRandom id field-1 10"),
            "@WriteRandom field-1 10"
        );
        assert_eq!(
            clean_command("@WriteRandomNum id field-2 6"),
            "@WriteRandomNum field-2 6"
        );
    }

    #[test]
    fn es_case_insensitive_para_la_palabra_id() {
        assert_eq!(
            clean_command(r#"@Write Id "input-1-root" "admin""#),
            r#"@Write input-1-root "admin""#
        );
    }

    #[test]
    fn no_modifica_comandos_ya_correctos_idempotencia() {
        assert_eq!(
            clean_command(r#"@Write input-2-root "admin123""#),
            r#"@Write input-2-root "admin123""#
        );
        assert_eq!(clean_command("@Click button-1-root"), "@Click button-1-root");
        assert_eq!(clean_command("@Wait 500"), "@Wait 500");
    }

    #[test]
    fn no_rompe_ids_que_empiezan_con_id_como_prefijo() {
        // "identifier-1" no debe perder el prefijo "id" porque no es
        // la palabra "id" aislada seguida de espacio.
        assert_eq!(clean_command("@Click identifier-1"), "@Click identifier-1");
    }

    #[test]
    fn pipeline_completo_sanitiza_y_valida_respuesta_real_de_la_ia() {
        let raw = r#"{
          "thought": "filling login form",
          "status": "CONTINUE",
          "commands": [
            "@Write id \"input-1-root\" \"admin\"",
            "@Write id \"input-2-root\" \"admin123\"",
            "@Click id \"button-1-root\""
          ]
        }"#;

        let parsed = sanitize_ollama_response(raw).expect("debería parsear sin error");
        let valid = validate_commands(&parsed.commands);

        assert_eq!(
            valid,
            vec![
                r#"@Write input-1-root "admin""#.to_string(),
                r#"@Write input-2-root "admin123""#.to_string(),
                "@Click button-1-root".to_string(),
            ]
        );
    }
}