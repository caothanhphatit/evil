use axum::{
    extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use tracing::{debug, error, warn};
use uuid::Uuid;

use std::time::Duration;

use crate::{
    simulation::{
        ClientCommand, ClientEnvelope, OriginalFlowSession, ServerEnvelope, ServerMessage,
        MAX_MESSAGE_BYTES, PROTOCOL_VERSION,
    },
    AppState,
};

use super::session::session_token;

pub async fn upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = session_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let ttl = Duration::from_secs(state.config.session.ttl_seconds);
    let player_token = match state.coordinator.resolve(token, ttl).await {
        Ok(Some(player)) => player,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state, token, player_token))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    session_token: Uuid,
    player_token: Uuid,
) {
    let session_id = Uuid::new_v4();
    let lease_ttl = Duration::from_millis(state.config.session.lease_ttl_ms);
    let lease = match state
        .coordinator
        .acquire_lease(player_token, session_id, lease_ttl)
        .await
    {
        Ok(Some(lease)) => lease,
        Ok(None) => {
            warn!(%session_id, %player_token, "rejected concurrent player session");
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
        Err(error) => {
            error!(%session_id, %player_token, %error, "failed to acquire player lease");
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let loaded = match state.repository.load_or_create(player_token).await {
        Ok(state) => state,
        Err(error) => {
            error!(%session_id, %player_token, %error, "failed to load player state");
            let _ = socket.send(Message::Close(None)).await;
            let _ = state.coordinator.release_lease(player_token, &lease).await;
            return;
        }
    };
    let mut revision = loaded.revision;
    let mut flow = OriginalFlowSession::from_state(loaded.state);
    let mut server_sequence = 0_u64;
    let mut last_client_sequence = 0_u64;
    if !send_message(
        &mut socket,
        session_id,
        &mut server_sequence,
        None,
        &ServerMessage::Welcome {
            player_token,
            session_id,
            snapshot: flow.snapshot(),
        },
    )
    .await
    {
        let _ = state.coordinator.release_lease(player_token, &lease).await;
        return;
    }

    let mut lease_interval = tokio::time::interval(lease_ttl / 3);
    lease_interval.tick().await;
    let mut visual_interval = tokio::time::interval(Duration::from_millis(200));
    visual_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    visual_interval.tick().await;

    loop {
        tokio::select! {
            _ = visual_interval.tick() => {
                let Some(snapshot) = flow.advance_visual_tick() else { continue };
                if !send_message(
                    &mut socket,
                    session_id,
                    &mut server_sequence,
                    None,
                    &ServerMessage::WorldUpdate { snapshot },
                ).await { break; }
            }
            _ = lease_interval.tick() => {
                match state.coordinator.renew_lease(player_token, &lease, lease_ttl).await {
                    Ok(true) => {}
                    Ok(false) | Err(_) => {
                        warn!(%session_id, %player_token, "player lease lost; closing session");
                        break;
                    }
                }
            }
            incoming = socket.next() => {
                let Some(incoming) = incoming else { break };
                match incoming {
                    Ok(Message::Text(text)) => {
                        match state.coordinator.allow_command(
                            session_token,
                            state.config.session.command_limit,
                            Duration::from_millis(state.config.session.command_window_ms),
                        ).await {
                            Ok(true) => {}
                            Ok(false) => {
                                warn!(%session_id, %player_token, "command rate limit exceeded");
                                break;
                            }
                            Err(error) => {
                                error!(%session_id, %player_token, %error, "command rate limiter unavailable");
                                break;
                            }
                        }
                        let envelope = match serde_json::from_str::<ClientEnvelope>(&text) {
                            Ok(envelope) => envelope,
                            Err(error) => {
                                warn!(%session_id, %error, "rejected invalid client command");
                                continue;
                            }
                        };
                        if envelope.version != PROTOCOL_VERSION
                            || envelope.session_id != Some(session_id)
                            || envelope.sequence != last_client_sequence + 1
                        {
                            warn!(
                                %session_id,
                                version = envelope.version,
                                sequence = envelope.sequence,
                                "rejected incompatible or out-of-order client envelope"
                            );
                            break;
                        }
                        last_client_sequence = envelope.sequence;
                        let correlation_id = envelope.correlation_id;
                        let command = envelope.payload;
                        if matches!(command, ClientCommand::RequestResync) {
                            if !send_message(
                                &mut socket,
                                session_id,
                                &mut server_sequence,
                                Some(correlation_id),
                                &ServerMessage::Resync { snapshot: flow.snapshot() },
                            ).await { break; }
                            continue;
                        }
                        if let Some(message) = flow.handle_command(command) {
                            match state.coordinator.renew_lease(player_token, &lease, lease_ttl).await {
                                Ok(true) => {}
                                Ok(false) | Err(_) => {
                                    warn!(%session_id, %player_token, "player lease lost before checkpoint");
                                    break;
                                }
                            }
                            match state.repository.persist(player_token, flow.state(), revision, lease.fence).await {
                                Ok(next_revision) => revision = next_revision,
                                Err(error) => {
                                    error!(%session_id, %player_token, %error, "failed to persist flow state");
                                    break;
                                }
                            }
                            if !send_message(&mut socket, session_id, &mut server_sequence, Some(correlation_id), &message).await { break; }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    Ok(Message::Ping(payload)) => {
                        if socket.send(Message::Pong(payload)).await.is_err() { break; }
                    }
                    _ => {}
                }
            }
        }
    }

    match state
        .coordinator
        .renew_lease(player_token, &lease, lease_ttl)
        .await
    {
        Ok(true) => {
            if let Err(error) = state
                .repository
                .persist(player_token, flow.state(), revision, lease.fence)
                .await
            {
                warn!(%session_id, %player_token, %error, "failed final player checkpoint");
            }
        }
        Ok(false) | Err(_) => {
            warn!(%session_id, %player_token, "skipped final checkpoint without player lease");
        }
    }
    debug!(%session_id, %player_token, "websocket session stopped");
    if let Err(error) = state.coordinator.release_lease(player_token, &lease).await {
        warn!(%session_id, %player_token, %error, "failed to release player lease");
    }
}

async fn send_message(
    socket: &mut WebSocket,
    session_id: Uuid,
    sequence: &mut u64,
    correlation_id: Option<Uuid>,
    message: &ServerMessage,
) -> bool {
    *sequence += 1;
    let envelope = ServerEnvelope {
        version: PROTOCOL_VERSION,
        sequence: *sequence,
        session_id,
        correlation_id,
        payload: message,
    };
    let payload = match serde_json::to_string(&envelope) {
        Ok(payload) => payload,
        Err(error) => {
            error!(%error, "failed to serialize server message");
            return false;
        }
    };
    socket.send(Message::Text(payload.into())).await.is_ok()
}
