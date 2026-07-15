// src-tauri/src/ws_server.rs

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_hdr_async;

use crate::AppState;
use crate::session::{PhaseRecord, TestSession};
use crate::storage;
use crate::dom_analyzer;
use crate::prompt_builder;
use crate::ai_client;
use crate::command_parser;

const BASE_PORT: u16 = 9999;
const MAX_PORT_TRIES: u16 = 10;
const ALLOWED_EXTENSION_ID: &str = "";

// Constantes para el timeout
const SESSION_TIMEOUT_SECONDS: u64 = 60;
const WATCHDOG_INTERVAL_SECONDS: u64 = 5;

// [DEBUG] Cambiar a true para permitir cualquier extensión (modo pruebas)
// Cambiar a false para modo producción (solo extensión oficial)
const IS_DEBUG: bool = true;

pub static EXTENSION_CONNECTED: AtomicBool = AtomicBool::new(false);

pub type Sender = broadcast::Sender<String>;

#[derive(Debug, Serialize, Deserialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(flatten)]
    payload: Value,
}

pub async fn start(
    app: tauri::AppHandle,
    broadcast_tx: broadcast::Sender<String>,
    state: Arc<AppState>,
) {
    if IS_DEBUG {
        println!("[SpectreQA] 🔧 MODO DEBUG activado: se aceptarán extensiones con cualquier ID");
    }

    let port = find_available_port(BASE_PORT).await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let broadcast_tx = Arc::new(broadcast_tx);

    let state_watchdog = Arc::clone(&state);
    let broadcast_tx_watchdog = Arc::clone(&broadcast_tx);
    tokio::spawn(async move {
        session_watchdog(state_watchdog, broadcast_tx_watchdog).await;
    });

    tokio::spawn(async move {
        start_http_discovery(BASE_PORT, port).await;
    });

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            println!("[SpectreQA] WS server escuchando en ws://127.0.0.1:{}", port);
            l
        }
        Err(e) => {
            eprintln!("[SpectreQA] No se pudo iniciar el WS server: {}", e);
            return;
        }
    };

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let tx = Arc::clone(&broadcast_tx);
        let app_handle = app.clone();
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            handle_connection(stream, peer_addr, tx, app_handle, state_clone).await;
        });
    }
}

async fn session_watchdog(state: Arc<AppState>, broadcast_tx: Arc<Sender>) {
    let mut interval = tokio::time::interval(Duration::from_secs(WATCHDOG_INTERVAL_SECONDS));
    
    loop {
        interval.tick().await;
        
        let should_timeout = {
            let session_guard = state.test_session.lock().await;
            if let Some(session) = session_guard.as_ref() {
                if session.is_navigating {
                    if session.is_navigation_timeout() {
                        println!(
                            "[SpectreQA] WATCHDOG: Timeout de navegación ({}s sin respuesta)",
                            session.navigation_timeout_secs
                        );
                        true
                    } else {
                        false
                    }
                } else if let Some(last_sent) = session.last_phase_sent_at {
                    last_sent.elapsed() > Duration::from_secs(SESSION_TIMEOUT_SECONDS)
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_timeout {
            println!(
                "[SpectreQA] WATCHDOG: Timeout de sesión detectado ({}s sin actividad)",
                SESSION_TIMEOUT_SECONDS
            );

            let timeout_msg = json!({
                "type": "EXECUTE_PHASE",
                "payload": {
                    "phase": 0,
                    "thought": format!(
                        "Timeout: El cliente no respondió en {} segundos. La sesión ha sido terminada.",
                        SESSION_TIMEOUT_SECONDS
                    ),
                    "status": "TIMEOUT",
                    "commands": [],
                    "actions": []
                }
            });
            let _ = broadcast_tx.send(timeout_msg.to_string());

            let mut session_guard = state.test_session.lock().await;
            *session_guard = None;
            println!("[SpectreQA] WATCHDOG: Sesión limpiada por timeout");
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    broadcast_tx: Arc<Sender>,
    app: tauri::AppHandle,
    state: Arc<AppState>,
) {
    let mut origin_ok = false;

    let ws_stream = accept_hdr_async(stream, |req: &Request, res: Response| {
        if IS_DEBUG {
            origin_ok = true;
            return Ok(res);
        }

        if let Some(origin) = req.headers().get("Origin") {
            if let Ok(origin_str) = origin.to_str() {
                let expected = format!("chrome-extension://{}", ALLOWED_EXTENSION_ID);
                origin_ok = origin_str == expected;
            }
        }
        Ok(res)
    })
    .await;

    let ws_stream = match ws_stream {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("[SpectreQA] Error en WS handshake desde {}: {}", peer_addr, e);
            return;
        }
    };

    if !origin_ok && !IS_DEBUG {
        eprintln!("[SpectreQA] Conexión rechazada desde {} (origen no permitido)", peer_addr);
        return;
    }

    if IS_DEBUG {
        println!("[SpectreQA] 🔧 Conexión aceptada en modo debug desde {}", peer_addr);
    } else {
        println!("[SpectreQA] Extensión conectada desde {}", peer_addr);
    }
    EXTENSION_CONNECTED.store(true, Ordering::Relaxed);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut handshake_done = false;

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        println!("[SpectreQA] Mensaje RAW recibido: {}", text);
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(parsed) => {
                                match parsed.msg_type.as_str() {
                                    "HANDSHAKE" => {
                                        let ext_id = parsed.payload
                                            .get("extensionId")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");

                                        if ext_id == ALLOWED_EXTENSION_ID || IS_DEBUG {
                                            handshake_done = true;
                                            EXTENSION_CONNECTED.store(true, Ordering::Relaxed);
                                            let _ = ws_sender.send(Message::Text(
                                                json!({ "type": "HANDSHAKE_ACK", "status": "ok" })
                                                    .to_string().into()
                                            )).await;
                                            if IS_DEBUG {
                                                println!("[SpectreQA] 🔧 Handshake aceptado en modo debug (ext: {})", ext_id);
                                            } else {
                                                println!("[SpectreQA] Handshake OK (ext: {})", ext_id);
                                            }
                                        } else {
                                            let _ = ws_sender.send(Message::Text(
                                                json!({
                                                    "type": "HANDSHAKE_ACK",
                                                    "status": "rejected",
                                                    "reason": "extension ID no permitido"
                                                }).to_string().into()
                                            )).await;
                                            eprintln!("[SpectreQA] Handshake rechazado (ext: {})", ext_id);
                                            break;
                                        }
                                    }
                                    "PING" if handshake_done => {
                                        let _ = ws_sender.send(Message::Text(
                                            json!({ "type": "PONG" }).to_string().into()
                                        )).await;
                                    }
                                    "START_TEST" if handshake_done => {
                                        println!("[SpectreQA] START_TEST recibido, procesando...");
                                        handle_start_test(&parsed.payload, &state, &broadcast_tx).await;
                                        println!("[SpectreQA] handle_start_test completado, TEST_STARTED enviado al broadcast");
                                    }
                                    "DOM_SNAPSHOT" if handshake_done => {
                                        handle_dom_snapshot(&parsed.payload, &state, &broadcast_tx).await;
                                    }
                                    "NAVIGATION_DETECTED" if handshake_done => {
                                        handle_navigation_detected(&parsed.payload, &state).await;
                                    }
                                    "TEST_STATUS_UPDATE" if handshake_done => {
                                        if let Some(status_str) = parsed.payload.get("status").and_then(|v| v.as_str()) {
                                            println!("[SpectreQA] Mensaje de estado recibido de la extensión: {}", status_str);

                                            if status_str == "ORCHESTRATOR_LOST" {
                                                handle_orchestrator_lost(&state).await;
                                            } else if status_str == "TEST_SUCCESS" {
                                                println!("[SpectreQA] Prueba terminada con ÉXITO por evento de ciclo de vida.");
                                                let mut session_guard = state.test_session.lock().await;
                                                *session_guard = None;
                                                println!("[SpectreQA] Sesión limpiada exitosamente.");
                                            } else if status_str == "TEST_ERROR" {
                                                handle_test_status_update(&parsed.payload, &state).await;
                                            } else {
                                                handle_test_status_update(&parsed.payload, &state).await;
                                            }
                                        } else {
                                            handle_test_status_update(&parsed.payload, &state).await;
                                        }
                                    }
                                    "AUDIT_EVENT" if handshake_done => {
                                        let _ = app.emit("audit-event", &parsed.payload);
                                    }
                                    "CLIENT_CONSOLE_ERROR" if handshake_done => {
                                        println!("[SpectreQA] console.error del cliente: {:?}", parsed.payload);
                                    }
                                    _ if !handshake_done => {
                                        eprintln!("[SpectreQA] Mensaje recibido sin handshake previo, ignorando.");
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => eprintln!("[SpectreQA] JSON inválido: {}", e),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        EXTENSION_CONNECTED.store(false, Ordering::Relaxed);
                        println!("[SpectreQA] Extensión desconectada ({})", peer_addr);
                        break;
                    }
                    Some(Err(e)) => {
                        EXTENSION_CONNECTED.store(false, Ordering::Relaxed);
                        eprintln!("[SpectreQA] Error WS: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            Ok(msg) = broadcast_rx.recv() => {
                if handshake_done {
                    if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                        EXTENSION_CONNECTED.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_start_test(
    payload: &Value,
    state: &Arc<AppState>,
    broadcast_tx: &Arc<Sender>,
) {
    let projects_base = match state.projects_base() {
        Ok(p) => p,
        Err(e) => {
            send_error(broadcast_tx, &format!("Error obteniendo ruta de proyectos: {}", e));
            return;
        }
    };

    let project_id = {
        let from_payload = payload
            .get("project_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(id) = from_payload {
            id
        } else {
            let active = state.active_project_id.lock().await;
            match active.clone() {
                Some(id) => id,
                None => {
                    send_error(broadcast_tx, "No hay proyecto activo. Activa un proyecto desde la app antes de iniciar.");
                    return;
                }
            }
        }
    };

    println!("[SpectreQA] handle_start_test: project_id = {}", project_id);

    let agent_md = match storage::read_agent_md(&projects_base, &project_id) {
        Ok(content) => content,
        Err(e) => {
            send_error(broadcast_tx, &format!("No se pudo leer AGENT.md: {}", e));
            return;
        }
    };

    let session = TestSession::new(project_id.clone(), agent_md, Vec::new());
    {
        let mut s = state.test_session.lock().await;
        *s = Some(session);
    }

    println!("[SpectreQA] Sesión iniciada para proyecto: {}", project_id);

    let ack = json!({
        "type": "TEST_STARTED",
        "project_id": project_id
    });
    println!("[SpectreQA] Enviando TEST_STARTED al broadcast. Suscriptores activos: {}", broadcast_tx.receiver_count());
    let result = broadcast_tx.send(ack.to_string());
    println!("[SpectreQA] Resultado del send: {:?}", result);
    println!("[SpectreQA] handle_start_test completado, TEST_STARTED enviado al broadcast");
}

async fn handle_navigation_detected(payload: &Value, state: &Arc<AppState>) {
    let mut session_guard = state.test_session.lock().await;
    if let Some(session) = session_guard.as_mut() {
        session.start_navigation();
        println!("[SpectreQA] Navegación detectada. Modo de espera activado.");
    }
}

async fn handle_orchestrator_lost(state: &Arc<AppState>) {
    let mut session_guard = state.test_session.lock().await;
    
    if let Some(session) = session_guard.as_mut() {
        if session.is_navigating {
            println!("[SpectreQA] Orquestador perdido durante navegación. Esperando reinyección...");
            return;
        } else {
            println!("[SpectreQA] Orquestador perdido sin navegación activa. Terminando prueba.");
            *session_guard = None;
        }
    }
}

async fn handle_dom_snapshot(
    payload: &Value,
    state: &Arc<AppState>,
    broadcast_tx: &Arc<Sender>,
) {
    println!("[SpectreQA] handle_dom_snapshot - payload recibido: {}", payload);

    let elements = if let Some(inner) = payload.get("payload") {
        if let Some(e) = inner.get("elements") {
            println!("[SpectreQA] elements encontrado dentro de payload");
            e.clone()
        } else {
            send_error(broadcast_tx, "DOM_SNAPSHOT: 'elements' no encontrado dentro de payload");
            return;
        }
    } else if let Some(e) = payload.get("elements") {
        println!("[SpectreQA] elements encontrado en la raíz");
        e.clone()
    } else {
        send_error(broadcast_tx, "DOM_SNAPSHOT: mensaje sin campo 'elements'");
        return;
    };

    let current_phase = if let Some(inner) = payload.get("payload") {
        inner.get("phase").and_then(|v| v.as_u64()).unwrap_or(0) as u32
    } else {
        payload.get("phase").and_then(|v| v.as_u64()).unwrap_or(0) as u32
    };

    let url = if let Some(inner) = payload.get("payload") {
        inner.get("url").and_then(|v| v.as_str()).unwrap_or("")
    } else {
        payload.get("url").and_then(|v| v.as_str()).unwrap_or("")
    }.to_string();

    let lifecycle_status = if let Some(inner) = payload.get("payload") {
        inner.get("lifecycleStatus").and_then(|v| v.as_str()).unwrap_or("READY")
    } else {
        payload.get("lifecycleStatus").and_then(|v| v.as_str()).unwrap_or("READY")
    };
    println!("[SpectreQA] lifecycleStatus recibido: {}", lifecycle_status);

    let projects_base = match state.projects_base() {
        Ok(p) => p,
        Err(e) => {
            send_error(broadcast_tx, &format!("Error obteniendo ruta de proyectos: {}", e));
            return;
        }
    };

    let mut session_guard = state.test_session.lock().await;
    let session = match session_guard.as_mut() {
        Some(s) => s,
        None => {
            send_error(broadcast_tx, "DOM_SNAPSHOT recibido sin sesión activa. Envía START_TEST primero.");
            return;
        }
    };

    if session.is_navigating {
        session.end_navigation();
        println!("[SpectreQA] Nueva página cargada y DOM recibido. Reanudando prueba...");
    }

    if lifecycle_status == "WAIT" {
        session.set_last_dom(elements.clone());
        session.last_url = Some(url.clone());
        session.mark_phase_sent();
        
        println!("[SpectreQA] Orquestador reinyectado con éxito. Página en estado WAIT. Pausando llamadas a la IA...");
        println!("[SpectreQA] Esperando evento CONTINUE para reanudar ejecución.");
        return;
    }

    println!("[SpectreQA] Procesando DOM_SNAPSHOT para Fase {} (URL: {})", current_phase, url);

    let previous_url = session.last_url.clone();
    let current_url = url.clone();
    session.last_url = Some(url);

    let dom_changed = if current_phase > 0 {
        if let Some(prev_elements) = &session.last_dom {
            dom_analyzer::has_dom_changed(prev_elements, &elements)
        } else {
            true
        }
    } else {
        true
    };

    if !dom_changed {
        let exceeded = session.register_no_change();
        if exceeded {
            println!(
                "[SpectreQA] DOM estancado tras {} reintentos en Fase {}. Terminando.",
                session.no_change_retries, current_phase
            );
            let msg = json!({
                "type": "EXECUTE_PHASE",
                "payload": {
                    "phase": current_phase,
                    "thought": "El DOM no cambió tras varios intentos.",
                    "status": "ERROR_NO_CHANGE",
                    "commands": [],
                    "actions": []
                }
            });
            let _ = broadcast_tx.send(msg.to_string());
            *session_guard = None;
            return;
        }

        println!(
            "[SpectreQA] DOM sin cambios en Fase {}. Reintento {}/3",
            current_phase, session.no_change_retries
        );

        tokio::time::sleep(std::time::Duration::from_millis(600)).await;

        let retry_msg = json!({
            "type": "EXECUTE_PHASE",
            "payload": {
                "phase": current_phase,
                "thought": format!("Esperando cambios en el DOM (intento {}/3)...", session.no_change_retries),
                "status": "CONTINUE",
                "commands": [],
                "actions": []
            }
        });
        let _ = broadcast_tx.send(retry_msg.to_string());
        return;
    } else {
        session.no_change_retries = 0;
    }

    session.set_last_dom(elements.clone());

    let agent_md = match storage::read_agent_md(&projects_base, &session.project_id) {
        Ok(content) => content,
        Err(e) => {
            send_error(broadcast_tx, &format!("No se pudo leer AGENT.md: {}", e));
            return;
        }
    };

    let prompt = prompt_builder::build_prompt(
        &agent_md,
        &session.history,
        current_phase,
        &elements,
        previous_url.as_deref(),
        Some(&current_url),
    );

    let ai_config = {
        let cfg = state.ai_config.lock().await;
        cfg.clone()
    };

    println!(
        "[SpectreQA] Llamando a IA (modo: {}, modelo: {}, fase {})...",
        ai_config.mode, ai_config.model, current_phase
    );

    drop(session_guard);

    let raw_response = match ai_client::call_ai(&ai_config, &prompt).await {
        Ok(r) => r,
        Err(e) => {
            send_error(broadcast_tx, &format!("Error con el proveedor de IA: {}", e));
            return;
        }
    };

    println!("[SpectreQA] Respuesta cruda de IA (fase {}):\n{}\n", current_phase, raw_response);

    let mut session_guard = state.test_session.lock().await;
    let session = match session_guard.as_mut() {
        Some(s) => s,
        None => {
            send_error(broadcast_tx, "La sesión se perdió mientras la IA respondía.");
            return;
        }
    };

    match command_parser::sanitize_response(&raw_response) {
        Ok(parsed) => {
            println!("[SpectreQA] Fase {}: status={}, commands={}", current_phase, parsed.status, parsed.commands.len());

            session.history.push(PhaseRecord {
                phase: current_phase,
                thought: parsed.thought.clone(),
                status: parsed.status.clone(),
                commands: parsed.commands.clone(),
            });

            let next_phase = current_phase + 1;

            let msg = serde_json::json!({
                "type": "EXECUTE_PHASE",
                "payload": {
                    "phase": next_phase,
                    "thought": parsed.thought,
                    "status": parsed.status,
                    "commands": parsed.commands.clone(),
                    "actions": parsed.commands
                }
            });
            
            if let Err(e) = broadcast_tx.send(msg.to_string()) {
                eprintln!("[SpectreQA] Error al enviar EXECUTE_PHASE al canal de transmisión: {}", e);
            }
        }
        Err(err) => {
            eprintln!("[SpectreQA] Error crítico sanitizando la respuesta de la IA: {}", err);
            send_error(broadcast_tx, &format!("Error al parsear comandos: {}", err));
        }
    }
}

async fn handle_test_status_update(payload: &Value, state: &Arc<AppState>) {
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    
    println!("[SpectreQA] Prueba terminada con status: {}", status);
    
    let mut session = state.test_session.lock().await;
    if let Some(sess) = session.as_ref() {
        if !sess.is_navigating {
            *session = None;
        } else {
            println!("[SpectreQA] Ignorando limpieza de sesión durante navegación");
        }
    } else {
        *session = None;
    }
}

fn send_error(broadcast_tx: &Arc<Sender>, message: &str) {
    eprintln!("[SpectreQA] Error enviado a la extensión: {}", message);
    let msg = json!({
        "type": "EXECUTE_PHASE",
        "payload": {
            "phase": 0,
            "thought": message,
            "status": "ERROR_NO_CHANGE",
            "commands": [],
            "actions": []
        }
    });
    let _ = broadcast_tx.send(msg.to_string());
}

async fn start_http_discovery(http_port: u16, ws_port: u16) {
    let addr: SocketAddr = format!("127.0.0.1:{}", http_port).parse().unwrap();
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(_) => return,
    };

    let body = format!("{{\"port\":{}}}", ws_port);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let resp = response.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    }
}

async fn find_available_port(base: u16) -> u16 {
    for i in 0..MAX_PORT_TRIES {
        let port = base + i;
        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        if TcpListener::bind(&addr).await.is_ok() {
            return port;
        }
    }
    base
}