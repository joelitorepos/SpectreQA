// src-tauri/src/lib.rs

mod ws_server;
mod session;
mod storage;
mod dom_analyzer;
mod command_parser;
mod prompt_builder;
mod ai_client;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use session::TestSession;

/**
 * Configuración de IA simplificada para SpectreQA.
 * Solo dos modos: local (Ollama) o cloud (backend real de SpectreQA).
 */
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AIConfig {
    pub mode:     String, // "local" | "cloud"
    pub model:    String, // Solo usado en modo local (ej: "llama3.2")
    pub base_url: String, // Local: "http://localhost:11434", Cloud: "https://app.spectreqa.com"
    /** Solo usado en modo cloud — la API key generada en el dashboard. */
    pub api_key:  String,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            mode:     "local".to_string(),
            model:    "llama3.2".to_string(),
            base_url: "http://localhost:11434".to_string(),
            api_key:  String::new(),
        }
    }
}

/**
 * Estado global compartido entre el hilo de Tauri y el hilo del WebSocket.
 */
pub struct AppState {
    pub event_tx: broadcast::Sender<String>,
    pub active_project_id: Mutex<Option<String>>,
    pub test_session: Mutex<Option<TestSession>>,
    pub ai_config: Mutex<AIConfig>,
    pub app_handle: tauri::AppHandle,
}

impl AppState {
    pub fn projects_base(&self) -> Result<PathBuf, String> {
        self.app_handle
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())
            .map(|p| p.join("projects"))
    }
}

fn projects_base(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("projects"))
}

/**
 * Recibe la configuración de IA desde React y la persiste a disco
 * (settings.json vía tauri_plugin_store), así sobrevive a reinicios de la
 * app — antes solo vivía en el Mutex en memoria y se perdía al cerrar.
 * Modo local: model + base_url (Ollama). Modo cloud: base_url + api_key.
 */
#[tauri::command]
async fn set_ai_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
    model: String,
    base_url: String,
    api_key: Option<String>, // Option: compatibilidad con llamadas que no lo mandan (modo local)
) -> Result<(), String> {
    let api_key = api_key.unwrap_or_default();
    println!("[SpectreQA] Modo IA: {}, modelo: {}, URL: {}", mode, model, base_url);

    let new_config = AIConfig {
        mode: mode.clone(),
        model: model.clone(),
        base_url: base_url.clone(),
        api_key: api_key.clone(),
    };

    {
        let mut config = state.ai_config.lock().await;
        *config = new_config;
    }

    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("ai_mode", serde_json::json!(mode));
    store.set("ai_model", serde_json::json!(model));
    store.set("ai_base_url", serde_json::json!(base_url));
    store.set("ai_api_key", serde_json::json!(api_key));
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

/**
 * Devuelve la config actual de IA al frontend — necesario para que la UI
 * de configuración pueda mostrar el estado real al cargar, en vez de
 * asumir siempre los defaults.
 */
#[tauri::command]
async fn get_ai_config(state: tauri::State<'_, Arc<AppState>>) -> Result<AIConfig, String> {
    Ok(state.ai_config.lock().await.clone())
}

/**
 * Activa la auditoría de un proyecto.
 */
#[tauri::command]
async fn set_active_project(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<(), String> {
    let mut active = state.active_project_id.lock().await;
    println!("[SpectreQA] Proyecto activo: {}", project_id);
    *active = Some(project_id);
    Ok(())
}

/**
 * Desactiva la auditoría y limpia la sesión activa.
 */
#[tauri::command]
async fn clear_active_project(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut active = state.active_project_id.lock().await;
    *active = None;
    let mut session = state.test_session.lock().await;
    *session = None;
    println!("[SpectreQA] Proyecto activo limpiado.");
    Ok(())
}

#[tauri::command]
async fn send_to_extension(
    state: tauri::State<'_, Arc<AppState>>,
    message_type: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    println!("[SpectreQA] send_to_extension: type={}", message_type);
    let msg = serde_json::json!({ "type": message_type, "payload": payload }).to_string();
    state.event_tx.send(msg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn extension_connected() -> bool {
    ws_server::EXTENSION_CONNECTED.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
async fn list_test_logs(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<Vec<session::TestLogSummary>, String> {
    let base = state.projects_base()?;
    storage::list_test_logs(&base, &project_id)
}

#[tauri::command]
async fn read_test_log(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    log_id: String,
) -> Result<session::TestLogEntry, String> {
    let base = state.projects_base()?;
    storage::read_test_log(&base, &project_id, &log_id)
}

#[tauri::command]
async fn delete_test_log(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    log_id: String,
) -> Result<(), String> {
    let base = state.projects_base()?;
    storage::delete_test_log(&base, &project_id, &log_id)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn create_agent_md(
    app: tauri::AppHandle,
    project_id: String,
    project_name: String,
    project_url: String,
    project_env: String,
) -> Result<(), String> {
    let path = projects_base(&app)?.join(&project_id);
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;

    let env_label = if project_env == "local" { "Local" } else { "Producción" };
    let content = format!(
        "# AGENT.md — {name}\n\n\
         ## Proyecto\n\
         - **URL:** {url}\n\
         - **Entorno:** {env}\n\n\
         ## Instrucciones para el agente de testing\n\n\
         Describe aquí qué debe probar el agente automáticamente.\n\n\
         ### Ejemplos\n\
         - Interactúa con todos los elementos interactivos que encuentres.\n\
         - Rellena los formularios con datos de prueba.\n\
         - Verifica que los botones respondan correctamente.\n\n\
         ## Notas adicionales\n\n\
         _(opcional)_\n",
        name = project_name,
        url = project_url,
        env = env_label,
    );

    fs::write(path.join("AGENT.md"), content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn read_agent_md(app: tauri::AppHandle, project_id: String) -> Result<String, String> {
    let path = projects_base(&app)?.join(&project_id).join("AGENT.md");
    fs::read_to_string(&path).map_err(|e| format!("No se pudo leer AGENT.md: {}", e))
}

#[tauri::command]
fn write_agent_md(app: tauri::AppHandle, project_id: String, content: String) -> Result<(), String> {
    let path = projects_base(&app)?.join(&project_id).join("AGENT.md");
    fs::write(&path, content).map_err(|e| format!("No se pudo guardar AGENT.md: {}", e))
}

#[tauri::command]
fn delete_project_dir(app: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let path = projects_base(&app)?.join(&project_id);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn run_project_commands(command: String, cwd: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let (shell, arg) = ("cmd", "/C");

    #[cfg(not(target_os = "windows"))]
    let (shell, arg) = ("sh", "-c");

    let mut cmd = std::process::Command::new(shell);
    
    cmd.arg(arg)
       .arg(&command)
       .current_dir(&cwd)
       .stdout(std::process::Stdio::null())
       .stderr(std::process::Stdio::null())
       .env_remove("PYTHONHOME")
       .env_remove("PYTHONPATH")
       .env_remove("LD_LIBRARY_PATH");

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error: {}", e)),
    }
}

/// Extrae un número de puerto del comando.
fn extract_port_from_command(command: &str) -> Option<u16> {
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            if num_str.len() >= 4 {
                if let Ok(port) = num_str.parse::<u16>() {
                    if port >= 1024 {
                        let preceded_by_dot = start > 0 && chars[start - 1] == '.';
                        if !preceded_by_dot {
                            return Some(port);
                        }
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

async fn is_port_in_use(port: u16) -> bool {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    let addr = format!("127.0.0.1:{}", port);
    timeout(Duration::from_millis(200), TcpStream::connect(&addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let (tx, _rx) = broadcast::channel::<String>(32);

            // Carga la config guardada de una sesión anterior, si existe.
            // Si el store o alguna clave no existen todavía (primera vez que
            // corre la app), cada campo cae a su default individual.
            let store = app.store("settings.json")?;
            let defaults = AIConfig::default();
            let initial_ai_config = AIConfig {
                mode: store.get("ai_mode").and_then(|v| v.as_str().map(String::from)).unwrap_or(defaults.mode),
                model: store.get("ai_model").and_then(|v| v.as_str().map(String::from)).unwrap_or(defaults.model),
                base_url: store.get("ai_base_url").and_then(|v| v.as_str().map(String::from)).unwrap_or(defaults.base_url),
                api_key: store.get("ai_api_key").and_then(|v| v.as_str().map(String::from)).unwrap_or(defaults.api_key),
            };
            println!("[SpectreQA] Config de IA cargada: modo={}, model={}", initial_ai_config.mode, initial_ai_config.model);

            let app_state = Arc::new(AppState {
                event_tx:          tx.clone(),
                active_project_id: Mutex::new(None),
                test_session:      Mutex::new(None),
                ai_config:         Mutex::new(initial_ai_config),
                app_handle:        handle.clone(),
            });

            app.manage(app_state.clone());

            tauri::async_runtime::spawn(async move {
                ws_server::start(handle, tx, app_state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            extension_connected,
            send_to_extension,
            set_ai_config,
            get_ai_config,
            set_active_project,
            clear_active_project,
            create_agent_md,
            read_agent_md,
            write_agent_md,
            delete_project_dir,
            run_project_commands,
            list_test_logs,
            read_test_log,
            delete_test_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}