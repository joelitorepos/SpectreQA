// src-tauri/src/prompt_builder.rs

use serde_json::Value;
use crate::session::PhaseRecord;

/**
 * Construye el prompt completo para Ollama sustituyendo los cuatro placeholders
 * del template en prompt.txt:
 *   {AGENT.md}              — objetivo general del proyecto
 *   {HISTORIAL_DE_FASES}    — fases anteriores formateadas
 *   {NUMERO_FASE}           — número de fase actual
 *   {JSON_SANITIZADO_DEL_DOM} — snapshot DOM compacto
 */
pub fn build_prompt(
    agent_md: &str,
    history: &[PhaseRecord],
    phase_num: u32,
    dom: &Value,
) -> String {
    let template = include_str!("../resources/prompt.txt");

    let history_str = format_history(history);
    let dom_str = serialize_dom(dom);

    let prompt = template
        .replace("{AGENT.md}", agent_md.trim())
        .replace("{HISTORIAL_DE_FASES}", &history_str)
        .replace("{NUMERO_FASE}", &phase_num.to_string())
        .replace("{JSON_SANITIZADO_DEL_DOM}", &dom_str);

    let prompt_preview = prompt.chars().take(100).collect::<String>();
    println!("[GlassTest] Prompt generado para fase {}:\n{}\n", phase_num, prompt_preview);

    println!("[GlassTest] Prompt completo:\n{}\n", prompt);
    prompt
}

/**
 * Formatea el historial de fases anteriores en texto legible para la IA.
 * Si no hay historial, devuelve un string indicando que es la primera fase.
 * Ejemplo de salida:
 *   Fase 0:
 *     Thought: Found login form, filling credentials.
 *     Status: CONTINUE
 *     Commands:
 *       - @Write input-text-1-login "admin"
 *       - @Write input-text-2-login "1234"
 *       - @Click button-1-login
 */
fn format_history(history: &[PhaseRecord]) -> String {
    if history.is_empty() {
        return "No previous phases. This is the first interaction.".to_string();
    }

    history
        .iter()
        .map(|record| {
            let commands_str = if record.commands.is_empty() {
                "  (no commands)".to_string()
            } else {
                record
                    .commands
                    .iter()
                    .map(|c| format!("    - {}", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            format!(
                "Phase {}:\n  Thought: {}\n  Status: {}\n  Commands:\n{}",
                record.phase,
                record.thought,
                record.status,
                commands_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/**
 * Serializa el snapshot DOM a JSON compacto (sin espacios extra).
 * Compacto para ahorrar tokens — la IA no necesita pretty print.
 */
fn serialize_dom(dom: &Value) -> String {
    dom.to_string()
}