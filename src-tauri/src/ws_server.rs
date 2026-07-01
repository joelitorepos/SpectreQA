// src-tauri/src/ws_server.rs

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_hdr_async;

use crate::{AppState, AIConfig};
use crate::session::{PhaseRecord, TestSession};
use crate::storage;
use crate::dom_analyzer;
use crate::prompt_builder;
use crate::ai_client;
use crate::command_parser;

const BASE_PORT: u16 = 9999;
const MAX_PORT_TRIES: u16 = 10;
const ALLOWED_EXTENSION_ID: &str = "eonjhhhdhlmlhbcmccenadolnkpingpb";

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
    let port = find_available_port(BASE_PORT).await;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let broadcast_tx = Arc::new(broadcast_tx);

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

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    broadcast_tx: Arc<Sender>,
    app: tauri::AppHandle,
    state: Arc<AppState>,
) {
    let mut origin_ok = false;

    let ws_stream = accept_hdr_async(stream, |req: &Request, res: Response| {
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

    if !origin_ok {
        eprintln!("[SpectreQA] Conexión rechazada desde {} (origen no permitido)", peer_addr);
        return;
    }

    println!("[SpectreQA] Extensión conectada desde {}", peer_addr);
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

                                        if ext_id == ALLOWED_EXTENSION_ID {
                                            handshake_done = true;
                                            EXTENSION_CONNECTED.store(true, Ordering::Relaxed);
                                            let _ = ws_sender.send(Message::Text(
                                                json!({ "type": "HANDSHAKE_ACK", "status": "ok" })
                                                    .to_string().into()
                                            )).await;
                                            println!("[SpectreQA] Handshake OK (ext: {})", ext_id);
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
                                    "TEST_STATUS_UPDATE" if handshake_done => {
                                        handle_test_status_update(&parsed.payload, &state).await;
                                    }
                                    "AUDIT_EVENT" if handshake_done => {
                                        let _ = app.emit("audit-event", &parsed.payload);
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

    // Siempre resetear el historial al iniciar una nueva prueba
    if let Err(e) = storage::reset_history(&projects_base, &project_id) {
        eprintln!("[SpectreQA] Error reseteando historial: {}", e);
    } else {
        println!("[SpectreQA] Historial resetado correctamente para proyecto: {}", project_id);
    }

    let history = storage::read_respuestas(&projects_base, &project_id).unwrap_or_default();
    println!("[SpectreQA] Historial cargado: {} fases", history.len());

    let session = TestSession::new(project_id.clone(), agent_md, history);
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

async fn handle_dom_snapshot(
    payload: &Value,
    state: &Arc<AppState>,
    broadcast_tx: &Arc<Sender>,
) {
    println!("[SpectreQA] handle_dom_snapshot - payload recibido: {}", payload);

    // Extraer elements (dentro de payload o en raíz)
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

    // Extraer phase y url
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

    println!("[SpectreQA] Procesando DOM_SNAPSHOT para Fase {} (URL: {})", current_phase, url);

    // Guardar la URL actual en la sesión (si no existe)
    if session.last_url.is_none() {
        session.last_url = Some(url.clone());
    }

    // Detectar si el objetivo se ha cumplido (cambio de página o desaparición de inputs)
    let is_logged_in = if let Some(prev_url) = &session.last_url {
        prev_url != &url && url.contains("welcome") // Ajusta según tu app
    } else {
        false
    };

    // También comprobar si ya no hay elementos que contengan "input" en su ID
    let has_inputs = elements.as_array().map_or(false, |arr| {
        arr.iter().any(|el| el.get("id").and_then(|id| id.as_str()).map_or(false, |id| id.contains("input")))
    });

    if (is_logged_in || !has_inputs) && current_phase > 0 {
        println!("[SpectreQA] Objetivo cumplido (login exitoso). Enviando SUCCESS.");
        let msg = json!({
            "type": "EXECUTE_PHASE",
            "payload": {
                "phase": current_phase + 1,
                "thought": "Login successful. Test completed.",
                "status": "SUCCESS",
                "commands": []
            }
        });
        let _ = broadcast_tx.send(msg.to_string());
        *session_guard = None; // Limpiar sesión
        return;
    }

    // Actualizar la URL guardada
    session.last_url = Some(url);

    // Validar si el DOM ha cambiado
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
                    "thought": "The DOM has not changed after multiple attempts. Aborting test execution.",
                    "status": "ERROR_NO_CHANGE",
                    "commands": []
                }
            });
            let _ = broadcast_tx.send(msg.to_string());
            return;
        }
        println!(
            "[SpectreQA] DOM sin cambios en Fase {}. Reintento {}/3",
            current_phase, session.no_change_retries
        );
        return;
    } else {
        session.no_change_retries = 0;
    }

    // Guardar snapshot
    if let Err(e) = storage::append_mapa(&projects_base, &session.project_id, elements.clone()) {
        eprintln!("[SpectreQA] Error guardando mapa fase {}: {}", current_phase, e);
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
    );

    let ai_config = {
        let cfg = state.ai_config.lock().await;
        cfg.clone()
    };

    println!(
        "[SpectreQA] Llamando a IA (provider: {}, modelo: {}, fase {})...",
        ai_config.provider, ai_config.model, current_phase
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

    let parsed = match command_parser::sanitize_ollama_response(&raw_response) {
        Ok(r) => r,
        Err(e) => {
            send_error(broadcast_tx, &format!("Error parseando respuesta de IA: {}", e));
            return;
        }
    };

    let valid_commands = command_parser::validate_commands(&parsed.commands);

    println!(
        "[SpectreQA] Fase {}: status={}, commands={}",
        current_phase, parsed.status, valid_commands.len()
    );

    // Evitar duplicados en el historial: comparar con el último registro
    let last_record = session.history.last();
    let is_duplicate = last_record.map_or(false, |last| {
        last.thought == parsed.thought && last.commands == valid_commands
    });

    if !is_duplicate {
        let record = PhaseRecord {
            phase: current_phase,
            thought: parsed.thought.clone(),
            status: parsed.status.clone(),
            commands: valid_commands.clone(),
        };
        session.add_phase(record.clone());
        if let Err(e) = storage::append_respuesta(&projects_base, &session.project_id, &record) {
            eprintln!("[SpectreQA] Error guardando respuesta fase {}: {}", current_phase, e);
        }
    } else {
        println!("[SpectreQA] Registro duplicado ignorado para fase {}", current_phase);
    }

    // Avanzar la fase interna
    session.advance_phase();
    let next_phase = session.current_phase; // Fase que se enviará al orquestador

    // Enviar comandos a la extensión usando la fase actualizada
    let msg = json!({
        "type": "EXECUTE_PHASE",
        "payload": {
            "phase": next_phase,
            "thought": parsed.thought,
            "status": parsed.status,
            "commands": valid_commands
        }
    });
    let _ = broadcast_tx.send(msg.to_string());
}

async fn handle_test_status_update(payload: &Value, state: &Arc<AppState>) {
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    println!("[SpectreQA] Prueba terminada con status: {}", status);
    let mut session = state.test_session.lock().await;
    *session = None;
}

fn send_error(broadcast_tx: &Arc<Sender>, message: &str) {
    eprintln!("[SpectreQA] Error enviado a la extensión: {}", message);
    let msg = json!({
        "type": "EXECUTE_PHASE",
        "payload": {
            "phase": 0,
            "thought": message,
            "status": "ERROR_NO_CHANGE",
            "commands": []
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