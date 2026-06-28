// src-tauri/src/ollama_client.rs

use serde_json::json;

const OLLAMA_URL: &str = "http://localhost:11434/api/generate";

/*
 * Envía el prompt a Ollama y devuelve la respuesta cruda como String.
 * Usa stream: false para recibir la respuesta completa en una sola llamada.
 * Si Ollama no está corriendo devuelve un error claro para que ws_server
 * pueda notificarlo a la extensión.
 */
pub async fn call_ollama(prompt: &str, model: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    });

    let response = client
        .post(OLLAMA_URL)
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

    /*
     * Ollama con stream:false devuelve { "response": "...", "done": true, ... }
     * Extraemos solo el campo "response" que contiene el texto generado.
     */
    data.get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Ollama no devolvió campo 'response'. Respuesta completa: {}", data))
}