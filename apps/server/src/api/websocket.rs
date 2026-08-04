use axum::{
    extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use tracing::{debug, error, info_span, warn, Instrument};
use uuid::Uuid;

use std::time::{Duration, Instant};

use crate::{
    identity::SessionTokenHash,
    simulation::{
        ClientCommand, ClientEnvelope, OriginalFlowSession, ServerEnvelope, ServerMessage,
        MAX_MESSAGE_BYTES, PROTOCOL_VERSION,
    },
    AppState,
};

use super::session::{resolve_player, session_token};

const ACTIVE_WORLD_CHECKPOINT_SECONDS: u64 = 5;
// A slow checkpoint must not consume the simulation frame budget. The next
// checkpoint retries the unchanged durable snapshot when this budget expires.
// Full-demo checkpoints measure about 115 ms after set-based stock persistence.
// Keep a small cancellation margin while retaining the separate 100 ms frame warning.
const WORLD_CHECKPOINT_BUDGET: Duration = Duration::from_millis(150);

pub async fn upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = session_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let ttl = Duration::from_secs(state.config.session.ttl_seconds);
    let player_token = match resolve_player(&state, token, ttl).await {
        Ok(Some(player)) => player,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| {
            handle_socket(
                socket,
                state,
                SessionTokenHash::from_token(token),
                player_token,
            )
        })
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    session_token_hash: SessionTokenHash,
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
    let mut flow = OriginalFlowSession::from_aggregate_with_content(
        loaded.state,
        state.config.simulation.seed,
        state.building_content.clone(),
    );
    let mut durable_state_dirty = false;
    let mut pending_operations = Vec::new();
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
    // Domain/UI projections are comparatively large. World motion is published
    // separately at the simulation cadence so these snapshots cannot stall it.
    let mut visual_interval = tokio::time::interval(Duration::from_secs(1));
    visual_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    visual_interval.tick().await;
    let simulation_step =
        Duration::from_nanos(1_000_000_000_u64 / u64::from(state.config.simulation.tick_rate));
    let mut simulation_interval = tokio::time::interval(simulation_step);
    simulation_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    simulation_interval.tick().await;

    loop {
        tokio::select! {
            _ = simulation_interval.tick() => {
                let elapsed_ns = u64::try_from(simulation_step.as_nanos()).unwrap_or(u64::MAX);
                let Some(result) = flow.advance_simulation_step(elapsed_ns) else { continue };
                durable_state_dirty = true;
                pending_operations.extend(result.operations);
                let checkpoint_ticks = 10_u64
                    .saturating_mul(ACTIVE_WORLD_CHECKPOINT_SECONDS)
                    .max(1);
                // Autonomous rewards stay ordered in memory and flush with the fixed
                // checkpoint cadence. Persisting immediately for every operation put
                // PostgreSQL latency directly into the 10 Hz movement/combat loop.
                let should_checkpoint = result.simulation_tick % checkpoint_ticks == 0;
                if should_checkpoint {
                    match state.coordinator.renew_lease(player_token, &lease, lease_ttl).await {
                        Ok(true) => {}
                        Ok(false) | Err(_) => {
                            warn!(%session_id, %player_token, "player lease lost before simulation checkpoint");
                            break;
                        }
                    }
                    let durable_state = flow.durable_state();
                    let persist_started = Instant::now();
                    match tokio::time::timeout(
                        WORLD_CHECKPOINT_BUDGET,
                        state.repository.persist(
                            player_token,
                            &durable_state,
                            revision,
                            lease.fence,
                            &pending_operations,
                        ),
                    ).await {
                        Ok(Ok(next_revision)) => {
                            let persist_elapsed = persist_started.elapsed();
                            if persist_elapsed > simulation_step {
                                warn!(
                                    %session_id,
                                    %player_token,
                                    elapsed_ms = persist_elapsed.as_millis(),
                                    "simulation checkpoint exceeded one world-frame budget"
                                );
                            }
                            revision = next_revision;
                            durable_state_dirty = false;
                            pending_operations.clear();
                        }
                        Ok(Err(error)) => {
                            error!(%session_id, %player_token, error = ?error, "failed to persist simulation state");
                            break;
                        }
                        Err(_) => {
                            warn!(
                                %session_id,
                                %player_token,
                                budget_ms = WORLD_CHECKPOINT_BUDGET.as_millis(),
                                "simulation checkpoint exceeded persistence budget; retrying"
                            );
                        }
                    }
                }
                if !send_message(
                    &mut socket,
                    session_id,
                    &mut server_sequence,
                    None,
                    &ServerMessage::WorldFrame { world: result.world },
                ).await { break; }
            }
            _ = visual_interval.tick() => {
                // Advance long-running town services without serializing a full
                // aggregate into the latency-sensitive movement loop. Commands
                // and explicit resyncs still publish complete domain snapshots.
                flow.advance_visual_clock_by(1_000);
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
                            session_token_hash,
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
                        if let ClientCommand::SubmitFarmReport { report } = &command {
                            if report.from_revision != revision
                                || report.elapsed_ms == 0
                                || report.elapsed_ms > 10_000
                                || !report.protected_claims.is_empty()
                            {
                                warn!(
                                    %session_id,
                                    %player_token,
                                    window_id = report.window_id,
                                    "rejected invalid farm report at queue ingress"
                                );
                                break;
                            }
                            if let Err(error) = state
                                .coordinator
                                .enqueue_farm_report(player_token, report)
                                .await
                            {
                                error!(%session_id, %player_token, %error, "farm report queue unavailable");
                                break;
                            }
                            if !send_message(
                                &mut socket,
                                session_id,
                                &mut server_sequence,
                                Some(correlation_id),
                                &ServerMessage::FarmReportQueued {
                                    window_id: report.window_id,
                                },
                            ).await { break; }
                            continue;
                        }
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
                        let result = {
                            let span = info_span!(
                                "authoritative_command",
                                %session_id,
                                %player_token,
                                %correlation_id,
                                revision,
                            );
                            let _entered = span.enter();
                            flow.handle_command_with_id(command, correlation_id)
                        };
                        if let Some(result) = result {
                            if let ServerMessage::IntentResult { intent, accepted, .. } = &result.message {
                                debug!(
                                    %session_id,
                                    %player_token,
                                    %correlation_id,
                                    intent,
                                    accepted,
                                    durable_state_changed = result.durable_state_changed,
                                    operation_count = result.operations.len(),
                                    "authoritative command handled"
                                );
                            }
                            if let ServerMessage::IntentResult {
                                intent,
                                accepted: false,
                                reason,
                                ..
                            } = &result.message
                            {
                                warn!(
                                    %session_id,
                                    %player_token,
                                    %correlation_id,
                                    intent,
                                    reason = reason.as_deref().unwrap_or("unspecified"),
                                    "authoritative intent rejected"
                                );
                            }
                            pending_operations.extend(result.operations);
                            if result.durable_state_changed || !pending_operations.is_empty() {
                                durable_state_dirty = true;
                                match state.coordinator.renew_lease(player_token, &lease, lease_ttl).await {
                                    Ok(true) => {}
                                    Ok(false) | Err(_) => {
                                        warn!(%session_id, %player_token, "player lease lost before checkpoint");
                                        break;
                                    }
                                }
                                let durable_state = flow.durable_state();
                                let persist_span = info_span!(
                                    "authoritative_persist",
                                    %session_id,
                                    %player_token,
                                    %correlation_id,
                                    expected_revision = revision,
                                    lease_fence = lease.fence,
                                    operation_count = pending_operations.len(),
                                );
                                match state.repository.persist(
                                    player_token,
                                    &durable_state,
                                    revision,
                                    lease.fence,
                                    &pending_operations,
                                ).instrument(persist_span).await {
                                    Ok(next_revision) => {
                                        revision = next_revision;
                                        durable_state_dirty = false;
                                        pending_operations.clear();
                                    }
                                    Err(error) => {
                                        error!(%session_id, %player_token, %error, "failed to persist flow state");
                                        break;
                                    }
                                }
                            }
                            if !send_message(&mut socket, session_id, &mut server_sequence, Some(correlation_id), &result.message).await { break; }
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

    if durable_state_dirty {
        match state
            .coordinator
            .renew_lease(player_token, &lease, lease_ttl)
            .await
        {
            Ok(true) => {
                if let Err(error) = state
                    .repository
                    .persist(
                        player_token,
                        &flow.durable_state(),
                        revision,
                        lease.fence,
                        &pending_operations,
                    )
                    .await
                {
                    warn!(%session_id, %player_token, error = ?error, "failed final player checkpoint");
                }
            }
            Ok(false) | Err(_) => {
                warn!(%session_id, %player_token, "skipped final checkpoint without player lease");
            }
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
