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

use session::TestSession;

/**
 * Configuración del proveedor de IA elegido por el usuario.
 * Equivalente al AIConfig del frontend (useSettings.ts).
 */
#[derive(Debug, Clone)]
pub struct AIConfig {
    pub provider: String,  // "ollama" | "openai" | "anthropic"
    pub model: String,
    pub api_key: String,
    pub base_url: String,  // solo relevante para ollama
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            api_key: String::new(),
            base_url: "http://localhost:11434".to_string(),
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
 * Recibe la configuración de IA desde React y la guarda en AppState.
 * Se llama al arrancar la app y cada vez que el usuario guarda cambios
 * en SettingsPage. Rust usará esta config en la próxima prueba.
 */
#[tauri::command]
async fn set_ai_config(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
) -> Result<(), String> {
    let mut config = state.ai_config.lock().await;
    println!("[GlassTest] AI config actualizada: provider={}, model={}", provider, model);
    *config = AIConfig { provider, model, api_key, base_url };
    Ok(())
}

/**
 * Activa la auditoría de un proyecto. Rust lo recordará cuando llegue START_TEST.
 */
#[tauri::command]
async fn set_active_project(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> Result<(), String> {
    let mut active = state.active_project_id.lock().await;
    println!("[GlassTest] Proyecto activo: {}", project_id);
    *active = Some(project_id);
    Ok(())
}

/**
 * Desactiva la auditoría y limpia la sesión activa si la hubiera.
 */
#[tauri::command]
async fn clear_active_project(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut active = state.active_project_id.lock().await;
    *active = None;
    let mut session = state.test_session.lock().await;
    *session = None;
    println!("[GlassTest] Proyecto activo limpiado.");
    Ok(())
}

#[tauri::command]
async fn send_to_extension(
    state: tauri::State<'_, Arc<AppState>>,
    message_type: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    println!("[GlassTest] send_to_extension: type={}", message_type);
    let msg = serde_json::json!({ "type": message_type, "payload": payload }).to_string();
    state.event_tx.send(msg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn extension_connected() -> bool {
    ws_server::EXTENSION_CONNECTED.load(std::sync::atomic::Ordering::Relaxed)
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

    let mapas_path = path.join("mapas.json");
    if !mapas_path.exists() {
        fs::write(&mapas_path, "[]").map_err(|e| e.to_string())?;
    }

    let respuestas_path = path.join("respuestas.json");
    if !respuestas_path.exists() {
        fs::write(&respuestas_path, "[]").map_err(|e| e.to_string())?;
    }

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
/// Detecta patrones como: "8000", ":8000", "port 8000", "PORT=8000", "-- --port 8000"
fn extract_port_from_command(command: &str) -> Option<u16> {
    // Buscar número de 4 dígitos que parezca puerto (1024–65535)
    let re_patterns = [
        r":(\d{4,5})",        // :8000
        r"port[= ](\d{4,5})", // port=8000 o port 8000
        r"PORT[= ](\d{4,5})", // PORT=8000
        r"\s(\d{4,5})\s*$",   // número al final del comando
        r"\s(\d{4,5})\s",     // número suelto en medio
    ];
 
    for pattern in &re_patterns {
        // Búsqueda manual sin regex para no añadir dependencia
        if let Some(port) = find_port_pattern(command, pattern) {
            if port >= 1024 {
                return Some(port);
            }
        }
    }
    None
}
 
fn find_port_pattern(command: &str, pattern: &str) -> Option<u16> {
    // Implementación simple sin regex: buscar dígitos consecutivos de 4-5 chars
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
                        // Verificar que no sea parte de una IP (precedido por punto)
                        let preceded_by_dot = start > 0 && chars[start - 1] == '.';
                        if !preceded_by_dot {
                            let _ = pattern; // parámetro usado para futura extensión
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

            let app_state = Arc::new(AppState {
                event_tx:          tx.clone(),
                active_project_id: Mutex::new(None),
                test_session:      Mutex::new(None),
                ai_config:         Mutex::new(AIConfig::default()),
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
            set_active_project,
            clear_active_project,
            create_agent_md,
            read_agent_md,
            write_agent_md,
            delete_project_dir,
            run_project_commands,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}