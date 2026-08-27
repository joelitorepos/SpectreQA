// src-tauri/src/ai_client.rs

use serde_json::{json, Value};
use crate::AIConfig;
use crate::session::PhaseRecord;
use crate::prompt_builder;

/// Timeout común para todas las peticiones a la IA (120 segundos)
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// Timeout para el backend cloud de SpectreQA (30 segundos — hay inferencia
/// real del lado del servidor, no es un eco; 10s se quedaba corto sobre todo
/// en cache-miss + latencia del proveedor de IA que esté detrás).
const CLOUD_TIMEOUT_SECS: u64 = 30;

/// Crea un cliente HTTP con timeout configurado y opciones por defecto.
fn create_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))
}

/**
 * API PÚBLICA
 * Único punto de entrada, sin importar el modo. La diferencia clave:
 *   - "local"  arma el prompt completo (build_prompt) y se lo manda a Ollama,
 *     como siempre.
 *   - "cloud"  NO arma el prompt aquí — manda los datos crudos (agent_md,
 *     historial, DOM ofuscado) al backend de SpectreQA, que arma su propio
 *     prompt del lado del servidor. Nunca construimos un prompt que el
 *     backend va a ignorar.
 * En ambos casos el texto devuelto sigue pasando por command_parser tal
 * cual — este módulo nunca valida ni limpia la respuesta, esa sigue siendo
 * responsabilidad exclusiva de command_parser.rs.
 */
pub async fn call_ai(
    config: &AIConfig,
    project_id: &str,
    agent_md: &str,
    history: &[PhaseRecord],
    phase_num: u32,
    dom: &Value,
    previous_url: Option<&str>,
    current_url: Option<&str>,
) -> Result<String, String> {
    match config.mode.as_str() {
        "local" => {
            let prompt = prompt_builder::build_prompt(
                agent_md, history, phase_num, dom, previous_url, current_url,
            );
            call_ollama(config, &prompt).await
        }
        "cloud" => call_spectreqa_cloud(config, project_id, agent_md, history, phase_num, dom).await,
        other => Err(format!("Modo de IA no soportado: {}", other)),
    }
}

/**
 * Avisa al backend cloud cómo terminó la corrida completa —
 * POST /api/v1/execute/result. SUCCESS confirma el progreso acumulado
 * como el nuevo cache (commitPending del lado del servidor); cualquier
 * otro resultado (error o detención manual) lo descarta (discardPending),
 * dejando intacto el cache de la última corrida que sí funcionó.
 *
 * Solo tiene sentido llamarlo en modo cloud — en modo local no hay cache
 * de servidor que confirmar. No consume cuota (el backend no lo cobra).
 * Los errores acá se devuelven para que el caller decida si loguear y
 * seguir — nunca deben tumbar el flujo de finalización de la sesión, el
 * log local ya se guardó de forma independiente antes de llamar a esto.
 */
pub async fn report_result(config: &AIConfig, project_id: &str, status: &str) -> Result<(), String> {
    let client = create_http_client(CLOUD_TIMEOUT_SECS)?;
    let url = format!("{}/api/v1/execute/result", config.base_url);

    let body = json!({
        "projectId": project_id,
        "status": status,
    });

    let response = client
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error conectando con el backend de SpectreQA: {}", e))?;

    if !response.status().is_success() {
        let status_code = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Backend de SpectreQA respondió {}: {}", status_code, text));
    }

    Ok(())
}

/** 
 * OLLAMA (MODO LOCAL)
 */
async fn call_ollama(config: &AIConfig, prompt: &str) -> Result<String, String> {
    let client = create_http_client(REQUEST_TIMEOUT_SECS)?;
    let url = format!("{}/api/generate", config.base_url);
    let body = json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.0
        }
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                "Ollama no está corriendo. Inicia Ollama antes de ejecutar una prueba.".to_string()
            } else {
                format!("Error conectando con Ollama: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama respondió con error {}: {}", status, text));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de Ollama: {}", e))?;

    data.get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Ollama no devolvió campo 'response'. Respuesta completa: {}", data))
}

/**
 * BACKEND CLOUD DE SPECTREQA — POST /api/v1/execute
 *
 * A diferencia de Ollama, este servicio arma su propio prompt del lado del
 * servidor (misma lógica que build_prompt, pero server-side) — por eso acá
 * mandamos los campos crudos, no un prompt ya interpolado. Reusamos
 * exactamente la misma ofuscación de inputs que usa el modo local
 * (prompt_builder::obfuscate_dom_inputs), porque el backend nunca debe
 * recibir valores literales de formularios.
 *
 * config.api_key es la key generada en el dashboard (Authorization: Bearer).
 * config.base_url debe ser algo como "https://app.spectreqa.com".
 *
 * Devuelve el JSON crudo como String — command_parser.rs es quien decide
 * qué hacer con thought/commands, este módulo no valida el contenido.
 */
async fn call_spectreqa_cloud(
    config: &AIConfig,
    project_id: &str,
    agent_md: &str,
    history: &[PhaseRecord],
    phase_num: u32,
    dom: &Value,
) -> Result<String, String> {
    let client = create_http_client(CLOUD_TIMEOUT_SECS)?;
    let url = format!("{}/api/v1/execute", config.base_url);

    // Mismo tratamiento del DOM que en modo local: clonar y ofuscar los
    // inputs antes de que salga de esta máquina.
    let mut dom_obfuscated = dom.clone();
    prompt_builder::obfuscate_dom_inputs(&mut dom_obfuscated);

    // history como string formateado — el backend espera exactamente este
    // shape (ver execution.schema.ts: history es z.string(), no un array).
    let history_str = prompt_builder::format_history(history);

    let body = json!({
        "projectId": project_id,
        "agentMd":   agent_md,
        "history":   history_str,
        "phaseNum":  phase_num,
        "mapData":   dom_obfuscated,
    });

    let response = client
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                "No se pudo conectar con el backend de SpectreQA. Verifica la URL o tu conexión.".to_string()
            } else {
                format!("Error conectando con el backend de SpectreQA: {}", e)
            }
        })?;

    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        let hint = match status.as_u16() {
            401 => " (API key inválida o revocada — genera una nueva en el dashboard)",
            403 => " (sin acceso al servicio en la nube — revisa tu plan o acceso anticipado)",
            // Antes decía "límite diario" — la cuota dejó de ser diaria,
            // ahora es mensual y apilable (ver quota.service.ts en el
            // backend). El texto detallado real (con los números exactos
            // de consumo/techo) ya viene en `text` más abajo, tal cual lo
            // arma quota.middleware.ts — este hint es solo un resumen corto.
            429 => " (tu cuota ha alcanzado el techo máximo — puedes comprar un plan adicional o esperar a que tu prueba de acceso anticipado se restablezca)",
            _ => "",
        };
        return Err(format!("Backend de SpectreQA respondió {}{}: {}", status, hint, text));
    }

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta del backend de SpectreQA: {}", e))?;

    if !data.is_object() {
        return Err("El backend de SpectreQA devolvió un JSON inválido".to_string());
    }

    Ok(data.to_string())
}