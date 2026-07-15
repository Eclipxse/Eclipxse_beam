use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, mpsc},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use webrtc::{
    api::{
        APIBuilder, interceptor_registry::register_default_interceptors, media_engine::MediaEngine,
    },
    data_channel::{
        RTCDataChannel, data_channel_message::DataChannelMessage,
        data_channel_state::RTCDataChannelState,
    },
    ice_transport::{ice_candidate::RTCIceCandidateInit, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
};

use crate::backend::{
    BackendInner, MAX_UPLOAD_BYTES, SharedFileInfo, TransferDirection, TransferInfo,
    TransferStatus, sanitize_filename, unique_destination,
};

const PEERJS_SERVER: &str = "wss://0.peerjs.com:443/peerjs";
const PEERJS_VERSION: &str = "1.5.5";
const WEBRTC_CHUNK_BYTES: usize = 32 * 1024;
const MAX_BUFFERED_BYTES: usize = 4 * 1024 * 1024;

type TransportResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
type ChannelSlot = Arc<RwLock<Option<Arc<RTCDataChannel>>>>;

#[derive(Clone, Copy, Debug)]
pub enum WebRtcCommand {
    SendQueued,
}

struct ActivePeer {
    remote_id: String,
    connection_id: String,
    connection: Arc<RTCPeerConnection>,
}

struct IncomingFile {
    id: String,
    size: u64,
    received: u64,
    destination: PathBuf,
    temporary: PathBuf,
    output: File,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ControlMessage {
    #[serde(rename = "hello")]
    Hello {
        #[serde(rename = "deviceName")]
        device_name: String,
    },
    #[serde(rename = "file-meta")]
    FileMeta {
        #[serde(rename = "transferId")]
        transfer_id: String,
        name: String,
        size: u64,
        #[serde(rename = "mimeType", default)]
        mime_type: String,
    },
    #[serde(rename = "file-complete")]
    FileComplete {
        #[serde(rename = "transferId")]
        transfer_id: String,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

pub fn start(
    peer_id: String,
    state: Arc<BackendInner>,
    mut commands: mpsc::UnboundedReceiver<WebRtcCommand>,
) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = run_signaling_session(&peer_id, state.clone(), &mut commands).await
            {
                state.mark_device_disconnected();
                state.set_error(format!(
                    "Secure pairing is reconnecting: {}",
                    friendly_error(error.as_ref())
                ));
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    });
}

async fn run_signaling_session(
    peer_id: &str,
    state: Arc<BackendInner>,
    commands: &mut mpsc::UnboundedReceiver<WebRtcCommand>,
) -> TransportResult<()> {
    let token = random_token(32);
    let websocket_url =
        format!("{PEERJS_SERVER}?key=peerjs&id={peer_id}&token={token}&version={PEERJS_VERSION}");
    let (websocket, _) = connect_async(websocket_url).await?;
    let (mut websocket_writer, mut websocket_reader) = websocket.split();
    let (signal_sender, mut signal_receiver) = mpsc::unbounded_channel::<String>();
    let channel_slot: ChannelSlot = Arc::new(RwLock::new(None));
    let incoming_file = Arc::new(Mutex::new(None::<IncomingFile>));
    let mut active_peer: Option<ActivePeer> = None;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                signal_sender.send(json!({ "type": "HEARTBEAT" }).to_string())?;
                let channel = channel_slot.read().expect("channel slot poisoned").clone();
                if let Some(channel) = channel
                    && channel.ready_state() == RTCDataChannelState::Open
                    && channel.send_text(json!({ "type": "ping" }).to_string()).await.is_ok()
                {
                    state.refresh_device();
                }
            }
            Some(outbound) = signal_receiver.recv() => {
                websocket_writer.send(Message::Text(outbound.into())).await?;
            }
            Some(command) = commands.recv() => {
                match command {
                    WebRtcCommand::SendQueued => {
                        let channel = channel_slot.read().expect("channel slot poisoned").clone();
                        if let Some(channel) = channel.filter(|channel| channel.ready_state() == RTCDataChannelState::Open) {
                            let send_state = state.clone();
                            tokio::spawn(async move {
                                if let Err(error) = send_selected_files(channel, send_state.clone()).await {
                                    send_state.set_error(format!("Could not send the selected files: {error}"));
                                }
                            });
                        } else {
                            state.set_error("Scan the QR code and connect your phone before sending.".into());
                        }
                    }
                }
            }
            incoming = websocket_reader.next() => {
                let message = incoming.ok_or("PeerJS signaling closed")??;
                let Message::Text(text) = message else { continue };
                let envelope: Value = serde_json::from_str(text.as_ref())?;
                let message_type = envelope.get("type").and_then(Value::as_str).unwrap_or_default();
                match message_type {
                    "OPEN" => {}
                    "OFFER" => {
                        let remote_id = envelope.get("src").and_then(Value::as_str).ok_or("offer has no source")?.to_owned();
                        let payload = envelope.get("payload").ok_or("offer has no payload")?;
                        if payload.get("type").and_then(Value::as_str) != Some("data")
                            || payload.get("serialization").and_then(Value::as_str) != Some("raw")
                        {
                            continue;
                        }
                        let connection_id = payload.get("connectionId").and_then(Value::as_str).ok_or("offer has no connection id")?.to_owned();
                        let device_name = payload
                            .pointer("/metadata/deviceName")
                            .and_then(Value::as_str)
                            .unwrap_or("Phone companion")
                            .chars()
                            .take(48)
                            .collect::<String>();
                        let offer: RTCSessionDescription = serde_json::from_value(
                            payload.get("sdp").cloned().ok_or("offer has no session description")?
                        )?;

                        if let Some(previous) = active_peer.take() {
                            let _ = previous.connection.close().await;
                        }
                        let previous_channel = channel_slot
                            .write()
                            .expect("channel slot poisoned")
                            .take();
                        if let Some(previous_channel) = previous_channel {
                            let _ = previous_channel.close().await;
                        }
                        state.mark_device_disconnected();

                        let connection = create_peer_connection().await?;
                        configure_peer_connection(
                            connection.clone(),
                            remote_id.clone(),
                            connection_id.clone(),
                            device_name,
                            signal_sender.clone(),
                            channel_slot.clone(),
                            incoming_file.clone(),
                            state.clone(),
                        );
                        connection.set_remote_description(offer).await?;
                        let answer = connection.create_answer(None).await?;
                        connection.set_local_description(answer).await?;
                        let local_description = connection
                            .local_description()
                            .await
                            .ok_or("WebRTC did not create an answer")?;
                        signal_sender.send(json!({
                            "type": "ANSWER",
                            "payload": {
                                "sdp": local_description,
                                "type": "data",
                                "connectionId": connection_id,
                            },
                            "dst": remote_id,
                        }).to_string())?;
                        active_peer = Some(ActivePeer {
                            remote_id,
                            connection_id,
                            connection,
                        });
                    }
                    "CANDIDATE" => {
                        let source = envelope.get("src").and_then(Value::as_str).unwrap_or_default();
                        let payload = envelope.get("payload").ok_or("candidate has no payload")?;
                        let connection_id = payload.get("connectionId").and_then(Value::as_str).unwrap_or_default();
                        if let Some(peer) = active_peer.as_ref().filter(|peer| peer.remote_id == source && peer.connection_id == connection_id) {
                            let candidate: RTCIceCandidateInit = serde_json::from_value(
                                payload.get("candidate").cloned().ok_or("candidate has no ICE value")?
                            )?;
                            peer.connection.add_ice_candidate(candidate).await?;
                        }
                    }
                    "LEAVE" => {
                        let source = envelope.get("src").and_then(Value::as_str).unwrap_or_default();
                        if active_peer.as_ref().is_some_and(|peer| peer.remote_id == source) {
                            if let Some(peer) = active_peer.take() {
                                let _ = peer.connection.close().await;
                            }
                            channel_slot.write().expect("channel slot poisoned").take();
                            state.mark_device_disconnected();
                        }
                    }
                    "ERROR" | "ID-TAKEN" | "INVALID-KEY" => {
                        return Err(format!("PeerJS returned {message_type}").into());
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn create_peer_connection() -> TransportResult<Arc<RTCPeerConnection>> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();
    let configuration = RTCConfiguration {
        ice_servers: vec![
            RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".into()],
                ..Default::default()
            },
            RTCIceServer {
                urls: vec![
                    "turn:eu-0.turn.peerjs.com:3478".into(),
                    "turn:us-0.turn.peerjs.com:3478".into(),
                ],
                username: "peerjs".into(),
                credential: "peerjsp".into(),
            },
        ],
        ..Default::default()
    };
    Ok(Arc::new(api.new_peer_connection(configuration).await?))
}

#[allow(clippy::too_many_arguments)]
fn configure_peer_connection(
    connection: Arc<RTCPeerConnection>,
    remote_id: String,
    connection_id: String,
    device_name: String,
    signal_sender: mpsc::UnboundedSender<String>,
    channel_slot: ChannelSlot,
    incoming_file: Arc<Mutex<Option<IncomingFile>>>,
    state: Arc<BackendInner>,
) {
    let candidate_remote = remote_id.clone();
    let candidate_connection = connection_id.clone();
    connection.on_ice_candidate(Box::new(move |candidate| {
        let signal_sender = signal_sender.clone();
        let remote_id = candidate_remote.clone();
        let connection_id = candidate_connection.clone();
        Box::pin(async move {
            let Some(candidate) = candidate else { return };
            let Ok(mut candidate) = candidate.to_json() else {
                return;
            };
            if candidate.sdp_mid.as_deref() == Some("") {
                candidate.sdp_mid = Some("0".into());
            }
            let _ = signal_sender.send(
                json!({
                    "type": "CANDIDATE",
                    "payload": {
                        "candidate": candidate,
                        "type": "data",
                        "connectionId": connection_id,
                    },
                    "dst": remote_id,
                })
                .to_string(),
            );
        })
    }));

    let state_change = state.clone();
    connection.on_peer_connection_state_change(Box::new(move |status| {
        let state = state_change.clone();
        Box::pin(async move {
            if matches!(
                status,
                RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Closed
                    | RTCPeerConnectionState::Disconnected
            ) {
                state.mark_device_disconnected();
            }
        })
    }));

    connection.on_data_channel(Box::new(move |channel| {
        let state = state.clone();
        let device_name = device_name.clone();
        let channel_slot = channel_slot.clone();
        let incoming_file = incoming_file.clone();
        Box::pin(async move {
            let message_state = state.clone();
            let message_channel = channel.clone();
            channel.on_message(Box::new(move |message| {
                let state = message_state.clone();
                let incoming_file = incoming_file.clone();
                let channel = message_channel.clone();
                Box::pin(async move {
                    handle_incoming_message(message, channel, state, incoming_file).await;
                })
            }));

            let open_state = state.clone();
            let open_channel = channel.clone();
            let open_slot = channel_slot.clone();
            channel.on_open(Box::new(move || {
                let state = open_state.clone();
                let channel = open_channel.clone();
                let channel_slot = open_slot.clone();
                let device_name = device_name.clone();
                Box::pin(async move {
                    *channel_slot.write().expect("channel slot poisoned") = Some(channel.clone());
                    state.mark_device_connected(device_name);
                    let _ = channel
                        .send_text(
                            json!({
                                "type": "hello",
                                "deviceName": "Eclipxse Beam Desktop",
                            })
                            .to_string(),
                        )
                        .await;
                })
            }));

            let close_state = state.clone();
            let close_slot = channel_slot.clone();
            channel.on_close(Box::new(move || {
                let state = close_state.clone();
                let channel_slot = close_slot.clone();
                Box::pin(async move {
                    channel_slot.write().expect("channel slot poisoned").take();
                    state.mark_device_disconnected();
                })
            }));
        })
    }));
}

async fn handle_incoming_message(
    message: DataChannelMessage,
    channel: Arc<RTCDataChannel>,
    state: Arc<BackendInner>,
    incoming_file: Arc<Mutex<Option<IncomingFile>>>,
) {
    if message.is_string {
        let Ok(text) = std::str::from_utf8(&message.data) else {
            return;
        };
        let Ok(control) = serde_json::from_str::<ControlMessage>(text) else {
            return;
        };
        match control {
            ControlMessage::Hello { device_name } => {
                let name = device_name.chars().take(48).collect::<String>();
                state.mark_device_connected(if name.is_empty() {
                    "Phone companion".into()
                } else {
                    name
                });
            }
            ControlMessage::FileMeta {
                transfer_id,
                name,
                size,
                mime_type,
            } => {
                let _ = mime_type;
                begin_incoming_file(transfer_id, name, size, state, incoming_file).await;
            }
            ControlMessage::FileComplete { transfer_id } => {
                complete_incoming_file(&transfer_id, state, incoming_file).await;
            }
            ControlMessage::Ping => {
                state.refresh_device();
                let _ = channel
                    .send_text(json!({ "type": "pong" }).to_string())
                    .await;
            }
            ControlMessage::Pong => state.refresh_device(),
        }
    } else {
        append_incoming_chunk(&message.data, state, incoming_file).await;
    }
}

async fn begin_incoming_file(
    transfer_id: String,
    name: String,
    size: u64,
    state: Arc<BackendInner>,
    incoming_file: Arc<Mutex<Option<IncomingFile>>>,
) {
    if size > MAX_UPLOAD_BYTES {
        state.set_error("The phone tried to send a file larger than 8 GB.".into());
        return;
    }
    let safe_name = sanitize_filename(&name);
    let root = state.download_root();
    if let Err(error) = tokio::fs::create_dir_all(&root).await {
        state.set_error(format!("Could not prepare the Downloads folder: {error}"));
        return;
    }
    let destination = unique_destination(&root, &safe_name).await;
    let safe_id = transfer_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(48)
        .collect::<String>();
    let temporary = root.join(format!(
        ".{}.part",
        if safe_id.is_empty() { "beam" } else { &safe_id }
    ));
    let output = match File::create(&temporary).await {
        Ok(output) => output,
        Err(error) => {
            state.set_error(format!("Could not create the received file: {error}"));
            return;
        }
    };

    let previous = incoming_file.lock().await.take();
    if let Some(previous) = previous {
        state.fail_transfer(
            &previous.id,
            "A newer phone transfer replaced this one.".into(),
        );
        let _ = tokio::fs::remove_file(previous.temporary).await;
    }
    state.begin_transfer(TransferInfo {
        id: transfer_id.clone(),
        name: safe_name,
        size,
        direction: TransferDirection::Receiving,
        status: TransferStatus::Transferring,
        progress: 0.0,
        saved_path: None,
        error: None,
    });
    *incoming_file.lock().await = Some(IncomingFile {
        id: transfer_id,
        size,
        received: 0,
        destination,
        temporary,
        output,
    });
}

async fn append_incoming_chunk(
    chunk: &Bytes,
    state: Arc<BackendInner>,
    incoming_file: Arc<Mutex<Option<IncomingFile>>>,
) {
    let mut guard = incoming_file.lock().await;
    let Some(file) = guard.as_mut() else { return };
    if file.received.saturating_add(chunk.len() as u64) > file.size {
        let failed = guard.take().expect("incoming file exists");
        drop(guard);
        state.fail_transfer(&failed.id, "The phone sent more data than expected.".into());
        let _ = tokio::fs::remove_file(failed.temporary).await;
        return;
    }
    if let Err(error) = file.output.write_all(chunk).await {
        let failed = guard.take().expect("incoming file exists");
        drop(guard);
        state.fail_transfer(
            &failed.id,
            format!("Could not save the received file: {error}"),
        );
        let _ = tokio::fs::remove_file(failed.temporary).await;
        return;
    }
    file.received += chunk.len() as u64;
    state.update_transfer(&file.id, file.received, TransferStatus::Transferring);
}

async fn complete_incoming_file(
    transfer_id: &str,
    state: Arc<BackendInner>,
    incoming_file: Arc<Mutex<Option<IncomingFile>>>,
) {
    let Some(mut file) = incoming_file.lock().await.take() else {
        return;
    };
    if file.id != transfer_id || file.received != file.size {
        state.fail_transfer(
            &file.id,
            "The phone transfer ended before all bytes arrived.".into(),
        );
        let _ = tokio::fs::remove_file(file.temporary).await;
        return;
    }
    if let Err(error) = file.output.flush().await {
        state.fail_transfer(
            &file.id,
            format!("Could not finish the received file: {error}"),
        );
        let _ = tokio::fs::remove_file(file.temporary).await;
        return;
    }
    drop(file.output);
    if let Err(error) = tokio::fs::rename(&file.temporary, &file.destination).await {
        state.fail_transfer(
            &file.id,
            format!("Could not move the received file: {error}"),
        );
        let _ = tokio::fs::remove_file(file.temporary).await;
        return;
    }
    state.complete_upload(&file.id, file.destination);
}

async fn send_selected_files(
    channel: Arc<RTCDataChannel>,
    state: Arc<BackendInner>,
) -> Result<(), String> {
    let files = state.selected_files();
    if files.is_empty() {
        return Err("Choose at least one file first.".into());
    }
    for file in files {
        send_file(channel.clone(), state.clone(), file).await?;
    }
    Ok(())
}

async fn send_file(
    channel: Arc<RTCDataChannel>,
    state: Arc<BackendInner>,
    shared: SharedFileInfo,
) -> Result<(), String> {
    state.begin_transfer(TransferInfo {
        id: shared.id.clone(),
        name: shared.name.clone(),
        size: shared.size,
        direction: TransferDirection::Sending,
        status: TransferStatus::Transferring,
        progress: 0.0,
        saved_path: None,
        error: None,
    });
    let mime_type = mime_guess::from_path(&shared.path)
        .first_or_octet_stream()
        .to_string();
    send_control(
        &channel,
        json!({
            "type": "file-meta",
            "transferId": shared.id,
            "name": shared.name,
            "size": shared.size,
            "mimeType": mime_type,
        }),
    )
    .await?;

    let mut input = File::open(&shared.path)
        .await
        .map_err(|error| format!("Could not open {}: {error}", shared.name))?;
    let mut sent = 0_u64;
    loop {
        let mut buffer = vec![0_u8; WEBRTC_CHUNK_BYTES];
        let read = input
            .read(&mut buffer)
            .await
            .map_err(|error| format!("Could not read {}: {error}", shared.name))?;
        if read == 0 {
            break;
        }
        while channel.buffered_amount().await > MAX_BUFFERED_BYTES {
            tokio::time::sleep(Duration::from_millis(10)).await;
            if channel.ready_state() != RTCDataChannelState::Open {
                let message = "The phone disconnected during the transfer.".to_owned();
                state.fail_transfer(&shared.id, message.clone());
                return Err(message);
            }
        }
        buffer.truncate(read);
        channel
            .send(&Bytes::from(buffer))
            .await
            .map_err(|error| format!("Could not send {}: {error}", shared.name))?;
        sent += read as u64;
        state.update_transfer(&shared.id, sent, TransferStatus::Transferring);
    }
    send_control(
        &channel,
        json!({ "type": "file-complete", "transferId": shared.id }),
    )
    .await?;
    state.update_transfer(&shared.id, sent, TransferStatus::Complete);
    Ok(())
}

async fn send_control(channel: &RTCDataChannel, value: Value) -> Result<(), String> {
    channel
        .send_text(value.to_string())
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn random_token(length: usize) -> String {
    use rand::{Rng, distr::Alphanumeric};
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn friendly_error(error: &(dyn Error + Send + Sync)) -> String {
    let message = error.to_string();
    if message.len() > 160 {
        format!("{}…", message.chars().take(159).collect::<String>())
    } else {
        message
    }
}
