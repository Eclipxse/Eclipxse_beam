use std::{
    collections::HashSet,
    error::Error,
    net::{IpAddr, Ipv4Addr, TcpListener, UdpSocket},
    path::{Path as FilePath, PathBuf},
    sync::{
        Arc, RwLock,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use async_stream::stream;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use bytes::Bytes;
use futures_util::StreamExt;
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const COMPANION_HTML: &str = include_str!("companion.html");
const MAX_UPLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const STREAM_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferDirection {
    Sending,
    Receiving,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferStatus {
    Waiting,
    Transferring,
    Complete,
    Failed,
}

#[derive(Clone, Debug)]
pub struct SharedFileInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct TransferInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub progress: f32,
    pub saved_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BackendSnapshot {
    pub files: Vec<SharedFileInfo>,
    pub transfers: Vec<TransferInfo>,
    pub device_name: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum BackendEvent {
    StateChanged,
}

#[derive(Default)]
struct SessionState {
    files: Vec<SharedFileInfo>,
    transfers: Vec<TransferInfo>,
    device_name: Option<String>,
    device_last_seen: Option<Instant>,
    last_error: Option<String>,
}

struct BackendInner {
    token: String,
    download_root: PathBuf,
    state: RwLock<SessionState>,
    events: Sender<BackendEvent>,
}

#[derive(Clone)]
pub struct BeamBackend {
    inner: Arc<BackendInner>,
    base_url: String,
    pairing_code: String,
    lan_reachable: bool,
}

impl BeamBackend {
    pub fn start() -> Result<(Self, Receiver<BackendEvent>), Box<dyn Error>> {
        let download_root = dirs::download_dir()
            .unwrap_or(std::env::current_dir()?)
            .join("Eclipxse Beam");
        Self::start_with_download_root(download_root)
    }

    fn start_with_download_root(
        download_root: PathBuf,
    ) -> Result<(Self, Receiver<BackendEvent>), Box<dyn Error>> {
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let local_ip = discover_local_ip().unwrap_or(Ipv4Addr::LOCALHOST);
        let lan_reachable = !local_ip.is_loopback();
        let token = random_token(28);
        let pairing_code = format!("{}-{}", &token[0..3], &token[3..6]).to_uppercase();
        let base_url = format!("http://{local_ip}:{port}/pair/{token}");
        let (events, receiver) = mpsc::channel();
        let inner = Arc::new(BackendInner {
            token,
            download_root,
            state: RwLock::new(SessionState::default()),
            events,
        });

        let server_state = inner.clone();
        thread::Builder::new()
            .name("beam-companion-server".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        server_state
                            .set_error(format!("Could not start the transfer engine: {error}"));
                        return;
                    }
                };

                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(error) => {
                            server_state
                                .set_error(format!("Could not open the companion port: {error}"));
                            return;
                        }
                    };
                    let app = companion_router(server_state.clone());
                    let presence_state = server_state.clone();
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            presence_state.expire_device_if_stale();
                        }
                    });
                    if let Err(error) = axum::serve(listener, app).await {
                        server_state.set_error(format!("The companion server stopped: {error}"));
                    }
                });
            })?;

        Ok((
            Self {
                inner,
                base_url,
                pairing_code,
                lan_reachable,
            },
            receiver,
        ))
    }

    pub fn pairing_url(&self) -> &str {
        &self.base_url
    }

    pub fn pairing_code(&self) -> &str {
        &self.pairing_code
    }

    pub fn lan_reachable(&self) -> bool {
        self.lan_reachable
    }

    pub fn snapshot(&self) -> BackendSnapshot {
        let state = self.inner.state.read().expect("backend state poisoned");
        BackendSnapshot {
            files: state.files.clone(),
            transfers: state.transfers.clone(),
            device_name: state.device_name.clone(),
            last_error: state.last_error.clone(),
        }
    }

    pub fn add_files(&self, paths: Vec<PathBuf>) {
        let mut state = self.inner.state.write().expect("backend state poisoned");
        let mut known = state
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<HashSet<_>>();

        for path in paths {
            if !known.insert(path.clone()) {
                continue;
            }
            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let id = random_token(12);
            let file = SharedFileInfo {
                id: id.clone(),
                name: name.to_owned(),
                path,
                size: metadata.len(),
            };
            state.transfers.push(TransferInfo {
                id,
                name: file.name.clone(),
                size: file.size,
                direction: TransferDirection::Sending,
                status: TransferStatus::Waiting,
                progress: 0.0,
                saved_path: None,
                error: None,
            });
            state.files.push(file);
        }
        drop(state);
        self.inner.notify();
    }

    pub fn remove_file(&self, index: usize) {
        let mut state = self.inner.state.write().expect("backend state poisoned");
        if index >= state.files.len() {
            return;
        }
        let removed = state.files.remove(index);
        state.transfers.retain(|transfer| transfer.id != removed.id);
        drop(state);
        self.inner.notify();
    }

    pub fn clear_files(&self) {
        let mut state = self.inner.state.write().expect("backend state poisoned");
        let outgoing_ids = state
            .files
            .iter()
            .map(|file| file.id.clone())
            .collect::<HashSet<_>>();
        state.files.clear();
        state
            .transfers
            .retain(|transfer| !outgoing_ids.contains(&transfer.id));
        drop(state);
        self.inner.notify();
    }
}

impl BackendInner {
    fn notify(&self) {
        let _ = self.events.send(BackendEvent::StateChanged);
    }

    fn set_error(&self, message: String) {
        self.state
            .write()
            .expect("backend state poisoned")
            .last_error = Some(message);
        self.notify();
    }

    fn mark_device_connected(&self, name: String) {
        let mut state = self.state.write().expect("backend state poisoned");
        let changed = state.device_name.as_deref() != Some(name.as_str());
        state.device_name = Some(name);
        state.device_last_seen = Some(Instant::now());
        state.last_error = None;
        drop(state);
        if changed {
            self.notify();
        }
    }

    fn expire_device_if_stale(&self) {
        let mut state = self.state.write().expect("backend state poisoned");
        let stale = state
            .device_last_seen
            .is_some_and(|last_seen| last_seen.elapsed() > Duration::from_secs(5));
        if !stale {
            return;
        }
        state.device_name = None;
        state.device_last_seen = None;
        drop(state);
        self.notify();
    }

    fn begin_transfer(&self, transfer: TransferInfo) {
        let mut state = self.state.write().expect("backend state poisoned");
        if let Some(existing) = state
            .transfers
            .iter_mut()
            .find(|existing| existing.id == transfer.id)
        {
            *existing = transfer;
        } else {
            state.transfers.push(transfer);
        }
        drop(state);
        self.notify();
    }

    fn update_transfer(&self, id: &str, sent: u64, status: TransferStatus) {
        let mut state = self.state.write().expect("backend state poisoned");
        let Some(transfer) = state
            .transfers
            .iter_mut()
            .find(|transfer| transfer.id == id)
        else {
            return;
        };
        let progress = if transfer.size == 0 {
            100.0
        } else {
            ((sent as f64 / transfer.size as f64) * 100.0).min(100.0) as f32
        };
        let changed_enough =
            (progress - transfer.progress).abs() >= 0.5 || status != transfer.status;
        transfer.progress = progress;
        transfer.status = status;
        if status == TransferStatus::Complete {
            transfer.progress = 100.0;
        }
        drop(state);
        if changed_enough || status == TransferStatus::Complete {
            self.notify();
        }
    }

    fn complete_upload(&self, id: &str, destination: PathBuf) {
        let mut state = self.state.write().expect("backend state poisoned");
        if let Some(transfer) = state
            .transfers
            .iter_mut()
            .find(|transfer| transfer.id == id)
        {
            transfer.progress = 100.0;
            transfer.status = TransferStatus::Complete;
            transfer.saved_path = Some(destination);
            transfer.error = None;
        }
        drop(state);
        self.notify();
    }

    fn fail_transfer(&self, id: &str, error: String) {
        let mut state = self.state.write().expect("backend state poisoned");
        if let Some(transfer) = state
            .transfers
            .iter_mut()
            .find(|transfer| transfer.id == id)
        {
            transfer.status = TransferStatus::Failed;
            transfer.error = Some(error.clone());
        }
        state.last_error = Some(error);
        drop(state);
        self.notify();
    }
}

fn companion_router(state: Arc<BackendInner>) -> Router {
    Router::new()
        .route("/pair/{token}", get(companion_page))
        .route("/api/session/{token}", get(session))
        .route("/api/file/{token}/{id}", get(download_file))
        .route("/api/upload/{token}", post(upload_file))
        .with_state(state)
}

async fn companion_page(
    State(state): State<Arc<BackendInner>>,
    Path(token): Path<String>,
) -> Result<Html<&'static str>, StatusCode> {
    authorize(&state, &token)?;
    Ok(Html(COMPANION_HTML))
}

#[derive(Serialize)]
struct SessionFile {
    id: String,
    name: String,
    size: u64,
}

#[derive(Serialize)]
struct SessionResponse {
    computer_name: &'static str,
    files: Vec<SessionFile>,
}

async fn session(
    State(state): State<Arc<BackendInner>>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, StatusCode> {
    authorize(&state, &token)?;
    state.mark_device_connected(device_name_from_headers(&headers));
    let files = state
        .state
        .read()
        .expect("backend state poisoned")
        .files
        .iter()
        .map(|file| SessionFile {
            id: file.id.clone(),
            name: file.name.clone(),
            size: file.size,
        })
        .collect();
    Ok(Json(SessionResponse {
        computer_name: "Eclipxse Beam",
        files,
    }))
}

async fn download_file(
    State(state): State<Arc<BackendInner>>,
    Path((token, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    authorize(&state, &token)?;
    state.mark_device_connected(device_name_from_headers(&headers));
    let shared = state
        .state
        .read()
        .expect("backend state poisoned")
        .files
        .iter()
        .find(|file| file.id == id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut file = tokio::fs::File::open(&shared.path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let stream_state = state.clone();
    let stream_id = shared.id.clone();
    let size = shared.size;
    state.begin_transfer(TransferInfo {
        id: shared.id.clone(),
        name: shared.name.clone(),
        size,
        direction: TransferDirection::Sending,
        status: TransferStatus::Transferring,
        progress: 0.0,
        saved_path: None,
        error: None,
    });

    let stream = stream! {
        let mut sent = 0_u64;
        loop {
            let mut buffer = vec![0_u8; STREAM_CHUNK_BYTES];
            let read = match file.read(&mut buffer).await {
                Ok(read) => read,
                Err(error) => {
                    stream_state.fail_transfer(&stream_id, format!("Could not read the shared file: {error}"));
                    yield Err::<Bytes, std::io::Error>(error);
                    break;
                }
            };
            if read == 0 {
                stream_state.update_transfer(&stream_id, sent, TransferStatus::Complete);
                break;
            }
            buffer.truncate(read);
            sent += read as u64;
            let status = if sent >= size {
                TransferStatus::Complete
            } else {
                TransferStatus::Transferring
            };
            stream_state.update_transfer(&stream_id, sent, status);
            yield Ok::<Bytes, std::io::Error>(Bytes::from(buffer));
        }
    };

    let content_type = mime_guess::from_path(&shared.path).first_or_octet_stream();
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        urlencoding::encode(&shared.name)
    );
    let mut response = Body::from_stream(stream).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&size.to_string()).expect("file length is a valid header"),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct UploadQuery {
    name: String,
}

async fn upload_file(
    State(state): State<Arc<BackendInner>>,
    Path(token): Path<String>,
    Query(query): Query<UploadQuery>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<UploadResponse>, StatusCode> {
    authorize(&state, &token)?;
    state.mark_device_connected(device_name_from_headers(&headers));
    let declared_size = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(StatusCode::LENGTH_REQUIRED)?;
    if declared_size > MAX_UPLOAD_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let name = sanitize_filename(&query.name);
    let id = random_token(12);
    let download_root = state.download_root.clone();
    tokio::fs::create_dir_all(&download_root)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let destination = unique_destination(&download_root, &name).await;
    let temporary = download_root.join(format!(".{id}.part"));
    let mut output = tokio::fs::File::create(&temporary)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state.begin_transfer(TransferInfo {
        id: id.clone(),
        name: name.clone(),
        size: declared_size,
        direction: TransferDirection::Receiving,
        status: TransferStatus::Transferring,
        progress: 0.0,
        saved_path: None,
        error: None,
    });

    let mut received = 0_u64;
    let mut stream = body.into_data_stream();
    while let Some(next) = stream.next().await {
        let chunk = match next {
            Ok(chunk) => chunk,
            Err(error) => {
                let message = format!("Upload interrupted: {error}");
                state.fail_transfer(&id, message);
                let _ = tokio::fs::remove_file(&temporary).await;
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        received += chunk.len() as u64;
        if received > MAX_UPLOAD_BYTES || received > declared_size {
            state.fail_transfer(&id, "The upload exceeded its declared size.".into());
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        if output.write_all(&chunk).await.is_err() {
            state.fail_transfer(&id, "Could not save the received file.".into());
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(StatusCode::INSUFFICIENT_STORAGE);
        }
        state.update_transfer(&id, received, TransferStatus::Transferring);
    }

    output
        .flush()
        .await
        .map_err(|_| StatusCode::INSUFFICIENT_STORAGE)?;
    drop(output);
    if received != declared_size {
        state.fail_transfer(
            &id,
            "The upload ended before the complete file arrived.".into(),
        );
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(StatusCode::BAD_REQUEST);
    }
    tokio::fs::rename(&temporary, &destination)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state.complete_upload(&id, destination.clone());

    Ok(Json(UploadResponse {
        saved_as: destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&name)
            .to_owned(),
    }))
}

#[derive(Serialize)]
struct UploadResponse {
    saved_as: String,
}

fn authorize(state: &BackendInner, token: &str) -> Result<(), StatusCode> {
    if token == state.token {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn device_name_from_headers(headers: &HeaderMap) -> String {
    let agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if agent.contains("iPhone") {
        "iPhone".into()
    } else if agent.contains("iPad") {
        "iPad".into()
    } else if agent.contains("Android") {
        "Android phone".into()
    } else {
        "Phone companion".into()
    }
}

fn discover_local_ip() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(("1.1.1.1", 80)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) => Some(ip),
        IpAddr::V6(_) => None,
    }
}

fn random_token(length: usize) -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn sanitize_filename(value: &str) -> String {
    let basename = FilePath::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("received-file");
    let cleaned = basename
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']);
    if cleaned.is_empty() {
        "received-file".into()
    } else {
        let shortened = cleaned.chars().take(180).collect::<String>();
        let stem = FilePath::new(&shortened)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit()
                && stem.as_bytes()[3] != b'0');
        if reserved {
            format!("_{shortened}")
        } else {
            shortened
        }
    }
}

async fn unique_destination(root: &FilePath, name: &str) -> PathBuf {
    let original = root.join(name);
    if !tokio::fs::try_exists(&original).await.unwrap_or(true) {
        return original;
    }
    let path = FilePath::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|value| value.to_str());
    for suffix in 1..10_000 {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = root.join(candidate_name);
        if !tokio::fs::try_exists(&candidate).await.unwrap_or(true) {
            return candidate;
        }
    }
    root.join(format!("{}-{name}", random_token(8)))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpStream,
        path::PathBuf,
        thread,
        time::Duration,
    };

    use super::{BeamBackend, TransferStatus, random_token, sanitize_filename};

    fn raw_request(method: &str, url: &str, body: &[u8]) -> Vec<u8> {
        let remainder = url.strip_prefix("http://").expect("test URL is HTTP");
        let (authority, path) = remainder.split_once('/').expect("test URL has a path");
        let mut stream = (0..50)
            .find_map(|_| match TcpStream::connect(authority) {
                Ok(stream) => Some(stream),
                Err(_) => {
                    thread::sleep(Duration::from_millis(20));
                    None
                }
            })
            .expect("companion server starts");
        write!(
            stream,
            "{method} /{path} HTTP/1.1\r\nHost: {authority}\r\nUser-Agent: Android test phone\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("request writes");
        stream.write_all(body).expect("request body writes");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).expect("response reads");
        response
    }

    fn raw_get(url: &str) -> Vec<u8> {
        raw_request("GET", url, &[])
    }

    #[test]
    fn strips_paths_and_windows_reserved_characters() {
        assert_eq!(sanitize_filename("../../bad:name?.png"), "bad_name_.png");
        assert_eq!(sanitize_filename("..\\..\\photo.jpg"), "photo.jpg");
    }

    #[test]
    fn replaces_empty_names() {
        assert_eq!(sanitize_filename("..."), "received-file");
        assert_eq!(sanitize_filename("CON.txt"), "_CON.txt");
    }

    #[test]
    fn companion_serves_pairing_page_and_selected_file() {
        let (backend, _events) = BeamBackend::start().expect("backend starts");
        let page = raw_get(backend.pairing_url());
        assert!(page.starts_with(b"HTTP/1.1 200"));
        assert!(
            page.windows(b"Beam Companion".len())
                .any(|part| part == b"Beam Companion")
        );

        let source: PathBuf = std::env::temp_dir().join(format!("beam-{}.txt", random_token(8)));
        fs::write(&source, b"real native beam payload").expect("fixture writes");
        backend.add_files(vec![source.clone()]);
        let snapshot = backend.snapshot();
        let file = snapshot.files.first().expect("file is selected");
        let (origin, token) = backend
            .pairing_url()
            .rsplit_once("/pair/")
            .expect("pairing URL contains token");
        let download_url = format!("{origin}/api/file/{token}/{}", file.id);
        let response = raw_get(&download_url);
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(
            response
                .windows(b"real native beam payload".len())
                .any(|part| part == b"real native beam payload")
        );
        assert_eq!(
            backend.snapshot().transfers[0].status,
            TransferStatus::Complete
        );
        let _ = fs::remove_file(source);
    }

    #[test]
    fn companion_saves_phone_uploads_to_the_configured_download_folder() {
        let download_root = std::env::temp_dir().join(format!("beam-upload-{}", random_token(8)));
        let (backend, _events) =
            BeamBackend::start_with_download_root(download_root.clone()).expect("backend starts");
        let (origin, token) = backend
            .pairing_url()
            .rsplit_once("/pair/")
            .expect("pairing URL contains token");
        let upload_url = format!("{origin}/api/upload/{token}?name=phone-note.txt");
        let response = raw_request("POST", &upload_url, b"hello from the phone");
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert_eq!(
            fs::read(download_root.join("phone-note.txt")).expect("upload was saved"),
            b"hello from the phone"
        );
        let snapshot = backend.snapshot();
        assert_eq!(snapshot.device_name.as_deref(), Some("Android phone"));
        assert_eq!(snapshot.transfers[0].status, TransferStatus::Complete);
        let _ = fs::remove_dir_all(download_root);
    }
}
