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
use crate::session::{PhaseRecord, TestSession, TestOutcome};
use crate::storage;
use crate::dom_analyzer;
use crate::ai_client;
use crate::command_parser;

const BASE_PORT: u16 = 9999;
const MAX_PORT_TRIES: u16 = 10;
const ALLOWED_EXTENSION_ID: &str = "eonjhhhdhlmlhbcmccenadolnkpingpb";

// Constantes para el timeout
const SESSION_TIMEOUT_SECONDS: u64 = 60;
const WATCHDOG_INTERVAL_SECONDS: u64 = 5;

// [DEBUG] Cambiar a true para permitir cualquier extensión (modo pruebas)
// Cambiar a false para modo producción (solo extensión oficial)
const IS_DEBUG: bool = false;

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
                if session.is_async_waiting {
                    // Espera controlada (PAUSE/WAIT manual o del QA): el watchdog
                    // no debe actuar, es justo para lo que existe este flag.
                    false
                } else if session.is_navigating {
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

            finish_session(&state, TestOutcome::Error).await;
            println!("[SpectreQA] WATCHDOG: Sesión finalizada por timeout");
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
                                        handle_test_status_update(&parsed.payload, &state).await;
                                    }
                                    "TEST_TERMINATED" if handshake_done => {
                                        handle_test_terminated(&parsed.payload, &state).await;
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

/**
 * Único punto donde una sesión termina de verdad: arma el log a partir del
 * historial de fases, lo guarda en disco, y limpia la sesión activa.
 * Se usa para los tres finales posibles: éxito, error, y detención manual.
 */
async fn finish_session(state: &Arc<AppState>, outcome: TestOutcome) {
    let mut session_guard = state.test_session.lock().await;
    let session = match session_guard.take() {
        Some(s) => s,
        None => {
            println!("[SpectreQA] finish_session({:?}) llamado sin sesión activa, nada que hacer.", outcome);
            return;
        }
    };
    drop(session_guard);

    let projects_base = match state.projects_base() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[SpectreQA] Sesión finalizada ({:?}) pero no se pudo obtener projects_base: {}", outcome, e);
            return;
        }
    };

    let entry = session.to_log_entry(outcome);
    match storage::save_test_log(&projects_base, &entry) {
        Ok(log_id) => println!("[SpectreQA] Sesión finalizada ({:?}). Log guardado: {}", outcome, log_id),
        Err(e) => eprintln!("[SpectreQA] Sesión finalizada ({:?}) pero no se pudo guardar el log: {}", outcome, e),
    }

    // Avisar al backend cloud cómo terminó la corrida — sin esto,
    // /execute/result nunca se llama del lado del servidor, y el cache
    // jamás se confirma (SUCCESS) ni se descarta (cualquier otro caso).
    // TERMINATED cuenta como fallo: una corrida interrumpida a mitad de
    // camino nunca debería quedar registrada como "la última corrida
    // buena" — mismo criterio que un ERROR real.
    let ai_config = { state.ai_config.lock().await.clone() };
    if ai_config.mode == "cloud" {
        let status = match outcome {
            TestOutcome::Success => "SUCCESS",
            TestOutcome::Error | TestOutcome::Terminated => "FAILURE",
        };

        match ai_client::report_result(&ai_config, &session.project_id, status).await {
            Ok(()) => println!("[SpectreQA] Resultado notificado al backend cloud: {} (proyecto {})", status, session.project_id),
            Err(e) => eprintln!("[SpectreQA] No se pudo notificar el resultado al backend cloud: {}", e),
        }
    }
}

async fn handle_orchestrator_lost(state: &Arc<AppState>) {
    let is_navigating = {
        let session_guard = state.test_session.lock().await;
        session_guard.as_ref().map(|s| s.is_navigating).unwrap_or(false)
    };

    if is_navigating {
        println!("[SpectreQA] Orquestador perdido durante navegación. Esperando reinyección...");
        return;
    }

    println!("[SpectreQA] Orquestador perdido sin navegación activa. Terminando prueba.");
    finish_session(state, TestOutcome::Error).await;
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

    let dom_changed = if let Some(prev_elements) = &session.last_dom {
        println!("[SpectreQA] comparando DOM");
        dom_analyzer::has_dom_changed(prev_elements, &elements)
    } else {
        true
    };

    if !dom_changed {
        session.register_no_change();
        println!(
            "[SpectreQA] DOM sin cambios en Fase {}. Terminando (sin reintentos).",
            current_phase
        );
        let no_change_message = "El DOM no cambió respecto al snapshot anterior.";
        let msg = json!({
            "type": "EXECUTE_PHASE",
            "payload": {
                "phase": current_phase,
                "thought": no_change_message,
                "status": "ERROR_NO_CHANGE",
                "commands": [],
                "actions": []
            }
        });
        let _ = broadcast_tx.send(msg.to_string());

        // Antes este mensaje solo se mandaba por broadcast (se veía en
        // pantalla y desaparecía) — nunca quedaba en session.history, así
        // que el log de la prueba terminaba con 0 fases registradas
        // ("Esta prueba terminó sin registrar fases."), sin ninguna pista
        // de qué pasó. Ahora se guarda como la última fase del log.
        session.add_phase(PhaseRecord {
            phase: current_phase,
            thought: no_change_message.to_string(),
            status: "ERROR_NO_CHANGE".to_string(),
            commands: vec![],
            message: Some(no_change_message.to_string()),
        });

        drop(session_guard);
        finish_session(state, TestOutcome::Error).await;
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

    let ai_config = {
        let cfg = state.ai_config.lock().await;
        cfg.clone()
    };

    println!(
        "[SpectreQA] Llamando a IA (modo: {}, modelo: {}, fase {})...",
        ai_config.mode, ai_config.model, current_phase
    );

    // Capturamos lo que necesitamos del historial ANTES de soltar el lock —
    // en modo "cloud" call_ai ya no recibe un prompt armado, arma/manda los
    // datos crudos internamente según el modo.
    let project_id = session.project_id.clone();
    let history_snapshot = session.history.clone();

    drop(session_guard);

    let raw_response = match ai_client::call_ai(
        &ai_config,
        &project_id,
        &agent_md,
        &history_snapshot,
        current_phase,
        &elements,
        previous_url.as_deref(),
        Some(&current_url),
    ).await {
        Ok(r) => r,
        Err(e) => {
            // CRÍTICO: un error del proveedor de IA (502, 404 de modelo, timeout,
            // etc.) es terminal, no algo para reintentar en silencio. Antes
            // solo se avisaba a la extensión pero la sesión seguía viva, así
            // que el siguiente DOM_SNAPSHOT volvía a llamar a la IA — y así
            // indefinidamente, sin backoff, gastando cuota real en cada vuelta.
            // finish_session() limpia la sesión: el próximo DOM_SNAPSHOT ya no
            // encuentra sesión activa y se corta ahí, sin tocar al proveedor.
            let error_message = format!("Error con el proveedor de IA: {}", e);
            send_error(broadcast_tx, &error_message);

            // Mismo motivo que en el caso de DOM sin cambios: sin esto, la
            // sesión termina con el historial tal cual estaba antes del
            // error (o vacío, si el error ocurrió en la fase 0 — justo el
            // caso del 429 de cuota) y el log no deja ningún rastro de por
            // qué terminó. Re-adquirimos el lock porque se soltó (drop
            // más arriba) antes de llamar a call_ai.
            {
                let mut session_guard = state.test_session.lock().await;
                if let Some(session) = session_guard.as_mut() {
                    session.add_phase(PhaseRecord {
                        phase: current_phase,
                        thought: error_message.clone(),
                        status: "ERROR_AI_PROVIDER".to_string(),
                        commands: vec![],
                        message: Some(error_message),
                    });
                }
            }

            finish_session(state, TestOutcome::Error).await;
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
                // Fase normal, sin mensaje especial que reportar — el
                // campo queda None y no se serializa (skip_serializing_if).
                message: None,
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

            // DIAGNÓSTICO: si esto muestra 2+ suscriptores de forma consistente,
            // confirma la teoría de conexiones WS duplicadas/zombie — el
            // EXECUTE_PHASE se manda a ambas pero solo una llega de verdad al
            // navegador. Con 1 suscriptor, el problema está en otro lado.
            println!(
                "[SpectreQA] Enviando EXECUTE_PHASE fase {} — suscriptores activos: {}",
                next_phase, broadcast_tx.receiver_count()
            );

            if let Err(e) = broadcast_tx.send(msg.to_string()) {
                eprintln!("[SpectreQA] Error al enviar EXECUTE_PHASE al canal de transmisión: {}", e);
            }
        }
        Err(err) => {
            eprintln!("[SpectreQA] Error crítico sanitizando la respuesta de la IA: {}", err);
            send_error(broadcast_tx, &format!("Error al parsear comandos: {}", err));
            // Soltamos el lock antes de finish_session (que toma su propio lock
            // sobre test_session) para no deadlockear. Mismo motivo que en el
            // error de proveedor: sin esto, el próximo snapshot reintenta.
            drop(session_guard);
            finish_session(state, TestOutcome::Error).await;
        }
    }
}

async fn handle_test_status_update(payload: &Value, state: &Arc<AppState>) {
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    let flag = payload
        .get("flag")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    println!("[SpectreQA] TEST_STATUS_UPDATE recibido: status={}, flag={}", status, flag);

    match status {
        "ORCHESTRATOR_LOST" => {
            handle_orchestrator_lost(state).await;
        }
        "TEST_SUCCESS" => {
            println!("[SpectreQA] Prueba terminada con ÉXITO.");
            finish_session(state, TestOutcome::Success).await;
        }
        "TEST_ERROR" => {
            println!("[SpectreQA] Prueba terminada con ERROR.");
            finish_session(state, TestOutcome::Error).await;
        }
        // WAIT y PAUSE de la extensión llegan aquí como "WAITING"/"PAUSED": son
        // pausas intencionales, no un fin de sesión. Se marca is_async_waiting
        // para que el watchdog no la mate por inactividad mientras dure.
        "WAITING" | "PAUSED" => {
            let mut session_guard = state.test_session.lock().await;
            if let Some(session) = session_guard.as_mut() {
                session.is_async_waiting = true;
                println!("[SpectreQA] Sesión en espera controlada ({}). Watchdog inhibido.", status);
            }
        }
        "RUNNING" => {
            let mut session_guard = state.test_session.lock().await;
            if let Some(session) = session_guard.as_mut() {
                session.is_async_waiting = false;
                session.mark_phase_sent(); // resetea el reloj del watchdog al reanudar
                println!("[SpectreQA] Sesión reanudada. Watchdog reactivado.");
            }
        }
        other => {
            // Antes: cualquier status no reconocido limpiaba la sesión entera.
            // Ahora: se ignora. Terminar la sesión debe ser explícito
            // (TEST_SUCCESS / TEST_ERROR / ORCHESTRATOR_LOST / TEST_TERMINATED),
            // no el comportamiento por defecto de un status desconocido.
            println!("[SpectreQA] TEST_STATUS_UPDATE con status no reconocido: '{}'. Se ignora.", other);
        }
    }
}

/// Detención voluntaria: el usuario le dio click a "Detener" en la extensión.
async fn handle_test_terminated(payload: &Value, state: &Arc<AppState>) {
    let reason = payload
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Sin razón especificada");

    println!("[SpectreQA] Prueba detenida por el usuario: {}", reason);
    finish_session(state, TestOutcome::Terminated).await;
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