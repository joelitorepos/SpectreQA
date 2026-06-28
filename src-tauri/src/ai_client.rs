// src-tauri/src/ai_client.rs

use serde_json::json;
use crate::AIConfig;

/// Timeout común para todas las peticiones a la IA (120 segundos)
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// Crea un cliente HTTP con timeout configurado y opciones por defecto.
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))
}

/** 
 * API PUBLICA
 */

/// Llama al proveedor de IA configurado con el prompt dado.
pub async fn call_ai(config: &AIConfig, prompt: &str) -> Result<String, String> {
    match config.provider.as_str() {
        "ollama" => call_ollama(config, prompt).await,
        "openai" => call_openai(config, prompt).await,
        "anthropic" => call_anthropic(config, prompt).await,
        other => Err(format!("Proveedor de IA no soportado: {}", other)),
    }
}

/** 
 * OLLAMA
 */
async fn call_ollama(config: &AIConfig, prompt: &str) -> Result<String, String> {
    let client = create_http_client()?;
    let url = format!("{}/api/generate", config.base_url);
    let body = json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false
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
 * OPENAI
 */
async fn call_openai(config: &AIConfig, prompt: &str) -> Result<String, String> {
    let client = create_http_client()?;
    let url = "https://api.openai.com/v1/chat/completions";

    let body = json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": "You are a helpful test automation assistant." },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.2
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error conectando con OpenAI: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI error {}: {}", status, text));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de OpenAI: {}", e))?;

    data["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("OpenAI no devolvió contenido esperado. Respuesta: {}", data))
}

/** 
 * ANTHROPIC
 */
async fn call_anthropic(config: &AIConfig, prompt: &str) -> Result<String, String> {
    let client = create_http_client()?;
    let url = "https://api.anthropic.com/v1/messages";

    let body = json!({
        "model": config.model,
        "max_tokens": 1024,
        "system": "You are a helpful test automation assistant.",
        "messages": [
            { "role": "user", "content": prompt }
        ]
    });

    let response = client
        .post(url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error conectando con Anthropic: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic error {}: {}", status, text));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta de Anthropic: {}", e))?;

    data["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Anthropic no devolvió texto esperado. Respuesta: {}", data))
}