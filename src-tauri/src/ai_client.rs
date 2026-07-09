// src-tauri/src/ai_client.rs

use serde_json::json;
use crate::AIConfig;

/// Timeout común para todas las peticiones a la IA (120 segundos)
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// Timeout para el servicio cloud FAST (10 segundos)
const FAST_CLOUD_TIMEOUT_SECS: u64 = 10;

/// Crea un cliente HTTP con timeout configurado y opciones por defecto.
fn create_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))
}

/** 
 * API PÚBLICA
 * Llama al modo de IA configurado con el prompt dado.
 */
pub async fn call_ai(config: &AIConfig, prompt: &str) -> Result<String, String> {
    match config.mode.as_str() {
        "local" => call_ollama(config, prompt).await,
        "cloud" => call_fast_cloud(config, prompt).await,
        other => Err(format!("Modo de IA no soportado: {}", other)),
    }
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
 * SERVICIO FAST CLOUD
 * Recibe un prompt y devuelve JSON con status + commands.
 * El prompt ya incluye todo el contexto (AGENT.md, historial, DOM).
 */
async fn call_fast_cloud(config: &AIConfig, prompt: &str) -> Result<String, String> {
    let client = create_http_client(FAST_CLOUD_TIMEOUT_SECS)?;
    let url = format!("{}/api/generate", config.base_url);
    
    let body = json!({
        "prompt": prompt,
        "model": config.model,  // Opcional, el servicio puede ignorarlo
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                "No se pudo conectar con el servicio FAST Cloud. Verifica la URL.".to_string()
            } else {
                format!("Error conectando con FAST Cloud: {}", e)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("FAST Cloud respondió con error {}: {}", status, text));
    }

    // El servicio devuelve JSON con: thought, status, commands
    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de FAST Cloud: {}", e))?;

    // Verificar que el JSON tenga el formato esperado
    if !data.is_object() {
        return Err("FAST Cloud devolvió un JSON inválido".to_string());
    }

    // Devolver el JSON como string para que command_parser lo procese
    Ok(data.to_string())
}