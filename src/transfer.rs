use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const SOCKET_BUF_SIZE: usize = 4 * 1024 * 1024;
const IO_BUF_SIZE: usize = 8 * 1024 * 1024;
const UI_UPDATE_INTERVAL_MS: u128 = 100;

fn tune_socket(stream: &TcpStream) {
    stream.set_nodelay(true).ok();
    // Increase OS-level socket buffers
    use std::os::windows::io::AsRawSocket;
    let raw = stream.as_raw_socket();
    unsafe {
        let buf_size = SOCKET_BUF_SIZE as i32;
        // SO_SNDBUF
        libc_setsockopt(raw, 0xFFFF, 0x1001, &buf_size);
        // SO_RCVBUF
        libc_setsockopt(raw, 0xFFFF, 0x1002, &buf_size);
    }
}

unsafe fn libc_setsockopt(socket: u64, level: i32, optname: i32, value: &i32) {
    #[allow(non_camel_case_types)]
    type SOCKET = u64;
    extern "system" {
        fn setsockopt(s: SOCKET, level: i32, optname: i32, optval: *const u8, optlen: i32) -> i32;
    }
    let ptr = value as *const i32 as *const u8;
    setsockopt(socket, level, optname, ptr, 4);
}

// ─── Transfer state shared with UI ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TransferInfo {
    pub filename: String,
    pub total_size: u64,
    pub transferred: u64,
    pub status: TransferStatus,
}

#[derive(Debug, Clone)]
pub struct TransferState {
    pub transfers: Vec<TransferInfo>,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub speed_bytes_per_sec: f64,
    pub is_sending: bool,
    pub is_receiving: bool,
    pub log_messages: Vec<String>,
}

impl Default for TransferState {
    fn default() -> Self {
        Self {
            transfers: Vec::new(),
            total_bytes: 0,
            transferred_bytes: 0,
            speed_bytes_per_sec: 0.0,
            is_sending: false,
            is_receiving: false,
            log_messages: Vec::new(),
        }
    }
}

impl TransferState {
    pub fn add_log(&mut self, msg: String) {
        if self.log_messages.len() > 200 {
            self.log_messages.remove(0);
        }
        self.log_messages.push(msg);
    }
}

#[derive(Debug, Clone)]
pub struct IncomingRequest {
    pub sender_name: String,
    pub sender_ip: String,
    pub files: Vec<IncomingFileInfo>,
    pub total_size: u64,
    pub request_id: u64,
    decision: mpsc::Sender<bool>,
}

impl IncomingRequest {
    pub fn respond(&self, accepted: bool) {
        let _ = self.decision.try_send(accepted);
    }
}

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Releases the single-active-transfer gate when the owning task ends.
struct ActiveGuard(Arc<AtomicBool>);

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// A single file in the incoming request preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingFileInfo {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct PendingFile {
    pub path: PathBuf,
    pub display_name: String,
    pub size: u64,
    pub is_dir: bool,
}

impl PendingFile {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let meta = std::fs::metadata(&path).ok()?;
        let is_dir = meta.is_dir();
        let size = if is_dir {
            dir_size(&path)
        } else {
            meta.len()
        };
        let display_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        Some(Self {
            path,
            display_name,
            size,
            is_dir,
        })
    }
}

/// Recursively calculate directory size.
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let meta = entry.metadata();
            if let Ok(m) = meta {
                if m.is_dir() {
                    total += dir_size(&entry.path());
                } else {
                    total += m.len();
                }
            }
        }
    }
    total
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferRequest {
    sender_name: String,
    files: Vec<IncomingFileInfo>,
    total_size: u64,
    file_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferResponse {
    accepted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub filename: String,
    pub size: u64,
    pub is_folder: bool,
    pub relative_path: String,
}

const END_MARKER_FILENAME: &str = "_END_";

async fn write_message<W: AsyncWriteExt + Unpin, T: Serialize>(writer: &mut W, msg: &T) -> Result<(), String> {
    let encoded = bincode::serialize(msg).map_err(|e| format!("Serialize: {e}"))?;
    let len = encoded.len() as u32;
    writer
        .write_all(&len.to_le_bytes())
        .await
        .map_err(|e| format!("Write length: {e}"))?;
    writer
        .write_all(&encoded)
        .await
        .map_err(|e| format!("Write body: {e}"))?;
    writer.flush().await.map_err(|e| format!("Flush: {e}"))?;
    Ok(())
}

async fn read_message<R: AsyncReadExt + Unpin, T: serde::de::DeserializeOwned>(reader: &mut R) -> Result<T, String> {
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| format!("Read length: {e}"))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    // Allow large manifests for folder transfers (many files). 256 MB is far
    // beyond anything a real file list would reach while still guarding against
    // corrupt/garbage length prefixes.
    const MAX_MESSAGE_SIZE: usize = 256 * 1024 * 1024;
    if len > MAX_MESSAGE_SIZE {
        return Err("Message too large".into());
    }
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("Read body: {e}"))?;
    bincode::deserialize(&buf).map_err(|e| format!("Deserialize: {e}"))
}

fn collect_files(pending: &[PendingFile]) -> Vec<(PathBuf, String, u64)> {
    let mut files = Vec::new();
    for item in pending {
        if item.is_dir {
            walk_dir(&item.path, &item.path, &mut files);
        } else {
            files.push((item.path.clone(), item.display_name.clone(), item.size));
        }
    }
    files
}

fn walk_dir(root: &Path, current: &Path, out: &mut Vec<(PathBuf, String, u64)>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(root.parent().unwrap_or(root))
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(m) = entry.metadata() {
                if m.is_dir() {
                    walk_dir(root, &path, out);
                } else {
                    out.push((path, rel, m.len()));
                }
            }
        }
    }
}

pub fn spawn_send(
    handle: &tokio::runtime::Handle,
    target_ip: String,
    target_port: u16,
    pending_files: Vec<PendingFile>,
    chunk_size: usize,
    device_name: String,
    state: Arc<Mutex<TransferState>>,
    cancel: Arc<AtomicBool>,
    transfer_active: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    handle.spawn(async move {
        if !transfer_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let mut st = state.lock().unwrap();
            st.add_log("Send blocked — another transfer is already running".into());
            ctx.request_repaint();
            return;
        }
        let _guard = ActiveGuard(transfer_active.clone());

        let files = collect_files(&pending_files);
        let total_bytes: u64 = files.iter().map(|(_, _, s)| s).sum();

        {
            let mut st = state.lock().unwrap();
            st.is_sending = true;
            st.total_bytes = total_bytes;
            st.transferred_bytes = 0;
            st.speed_bytes_per_sec = 0.0;
            st.transfers = files
                .iter()
                .map(|(_, rel, size)| TransferInfo {
                    filename: rel.clone(),
                    total_size: *size,
                    transferred: 0,
                    status: TransferStatus::Pending,
                })
                .collect();
            st.add_log(format!("Connecting to {}:{}...", target_ip, target_port));
        }
        ctx.request_repaint();

        let addr = format!("{target_ip}:{target_port}");
        let stream = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                let mut st = state.lock().unwrap();
                st.is_sending = false;
                st.add_log(format!("Connection failed: {e}"));
                for t in &mut st.transfers {
                    t.status = TransferStatus::Failed(e.to_string());
                }
                ctx.request_repaint();
                return;
            }
            Err(_) => {
                let mut st = state.lock().unwrap();
                st.is_sending = false;
                st.add_log("Connection timed out".into());
                ctx.request_repaint();
                return;
            }
        };

        tune_socket(&stream);

        let (read_half, write_half) = stream.into_split();
        let mut writer = BufWriter::with_capacity(IO_BUF_SIZE, write_half);
        let mut reader = BufReader::with_capacity(4096, read_half);

        {
            let mut st = state.lock().unwrap();
            st.add_log(format!("Connected to {addr}. Requesting transfer..."));
        }
        ctx.request_repaint();

        let request = TransferRequest {
            sender_name: device_name,
            files: files
                .iter()
                .map(|(_, rel, size)| IncomingFileInfo {
                    name: rel.clone(),
                    size: *size,
                })
                .collect(),
            total_size: total_bytes,
            file_count: files.len() as u32,
        };

        if let Err(e) = write_message(&mut writer, &request).await {
            let mut st = state.lock().unwrap();
            st.is_sending = false;
            st.add_log(format!("Failed to send transfer request: {e}"));
            ctx.request_repaint();
            return;
        }

        {
            let mut st = state.lock().unwrap();
            st.add_log("Waiting for receiver to accept...".into());
        }
        ctx.request_repaint();

        let response: TransferResponse = match tokio::time::timeout(
            std::time::Duration::from_secs(120), 
            read_message(&mut reader),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let mut st = state.lock().unwrap();
                st.is_sending = false;
                st.add_log(format!("Failed to read response: {e}"));
                for t in &mut st.transfers {
                    t.status = TransferStatus::Failed("No response".into());
                }
                ctx.request_repaint();
                return;
            }
            Err(_) => {
                let mut st = state.lock().unwrap();
                st.is_sending = false;
                st.add_log("Receiver did not respond in time".into());
                for t in &mut st.transfers {
                    t.status = TransferStatus::Failed("Timed out".into());
                }
                ctx.request_repaint();
                return;
            }
        };

        if !response.accepted {
            let mut st = state.lock().unwrap();
            st.is_sending = false;
            st.add_log("Transfer declined by receiver".into());
            for t in &mut st.transfers {
                t.status = TransferStatus::Cancelled;
            }
            ctx.request_repaint();
            return;
        }

        {
            let mut st = state.lock().unwrap();
            st.add_log(format!("Transfer accepted! Sending {} files...", files.len()));
        }
        ctx.request_repaint();

        let start_time = Instant::now();
        let mut total_sent: u64 = 0;

        for (i, (path, rel_path, size)) in files.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                let mut st = state.lock().unwrap();
                st.is_sending = false;
                st.add_log("Transfer cancelled by user".into());
                for t in st.transfers.iter_mut().skip(i) {
                    t.status = TransferStatus::Cancelled;
                }
                ctx.request_repaint();
                return;
            }

            {
                let mut st = state.lock().unwrap();
                st.transfers[i].status = TransferStatus::InProgress;
            }
            ctx.request_repaint();

            let meta = FileMetadata {
                filename: path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size: *size,
                is_folder: false,
                relative_path: rel_path.clone(),
            };

            if let Err(e) = write_message(&mut writer, &meta).await {
                let mut st = state.lock().unwrap();
                st.transfers[i].status = TransferStatus::Failed(e.clone());
                st.add_log(format!("Failed to send metadata for {rel_path}: {e}"));
                st.is_sending = false;
                ctx.request_repaint();
                return;
            }

            let file = match tokio::fs::File::open(&path).await {
                Ok(f) => f,
                Err(e) => {
                    let mut st = state.lock().unwrap();
                    st.transfers[i].status = TransferStatus::Failed(e.to_string());
                    st.add_log(format!("Failed to open {rel_path}: {e}"));
                    st.is_sending = false;
                    ctx.request_repaint();
                    return;
                }
            };

            let mut file_reader = BufReader::with_capacity(IO_BUF_SIZE, file);
            let mut file_sent: u64 = 0;
            let mut buf = vec![0u8; chunk_size];
            let mut last_ui_update = Instant::now();

            loop {
                if cancel.load(Ordering::Relaxed) {
                    let mut st = state.lock().unwrap();
                    st.is_sending = false;
                    st.transfers[i].status = TransferStatus::Cancelled;
                    st.add_log("Transfer cancelled".into());
                    ctx.request_repaint();
                    return;
                }

                let n = match file_reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        let mut st = state.lock().unwrap();
                        st.transfers[i].status = TransferStatus::Failed(e.to_string());
                        st.add_log(format!("Read error on {rel_path}: {e}"));
                        st.is_sending = false;
                        ctx.request_repaint();
                        return;
                    }
                };

                if let Err(e) = writer.write_all(&buf[..n]).await {
                    let mut st = state.lock().unwrap();
                    st.transfers[i].status = TransferStatus::Failed(e.to_string());
                    st.add_log(format!("Write error: {e}"));
                    st.is_sending = false;
                    ctx.request_repaint();
                    return;
                }

                file_sent += n as u64;
                total_sent += n as u64;

                if last_ui_update.elapsed().as_millis() >= UI_UPDATE_INTERVAL_MS {
                    let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
                    {
                        let mut st = state.lock().unwrap();
                        st.transfers[i].transferred = file_sent;
                        st.transferred_bytes = total_sent;
                        st.speed_bytes_per_sec = total_sent as f64 / elapsed;
                    }
                    ctx.request_repaint();
                    last_ui_update = Instant::now();
                }
            }

            {
                let mut st = state.lock().unwrap();
                st.transfers[i].status = TransferStatus::Complete;
                st.transfers[i].transferred = *size;
                st.add_log(format!("Sent: {rel_path}"));
            }
            ctx.request_repaint();
        }

        let end_meta = FileMetadata {
            filename: END_MARKER_FILENAME.into(),
            size: 0,
            is_folder: false,
            relative_path: String::new(),
        };
        let _ = write_message(&mut writer, &end_meta).await;
        let _ = writer.flush().await;

        let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
        let speed_mb = (total_sent as f64 / elapsed) / (1024.0 * 1024.0);
        {
            let mut st = state.lock().unwrap();
            st.is_sending = false;
            st.speed_bytes_per_sec = total_sent as f64 / elapsed;
            st.add_log(format!(
                "Transfer complete! {:.1} MB in {:.1}s ({:.1} MB/s)",
                total_sent as f64 / (1024.0 * 1024.0),
                elapsed,
                speed_mb
            ));
        }
        ctx.request_repaint();
    });
}


/// Spawn the TCP receive server.
pub fn spawn_receiver(
    handle: &tokio::runtime::Handle,
    port: u16,
    download_dir: PathBuf,
    chunk_size: usize,
    state: Arc<Mutex<TransferState>>,
    cancel: Arc<AtomicBool>,
    incoming_request: Arc<Mutex<Option<IncomingRequest>>>,
    transfer_active: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    handle.spawn(async move {
        let listener = match TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind TCP server on port {port}: {e}");
                let mut st = state.lock().unwrap();
                st.add_log(format!("TCP server bind error: {e}"));
                ctx.request_repaint();
                return;
            }
        };

        tracing::info!("TCP receive server listening on port {port}");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("Incoming transfer from {addr}");
                    let dl = download_dir.clone();
                    let st = state.clone();
                    let c = cancel.clone();
                    let ctx2 = ctx.clone();
                    let ir = incoming_request.clone();
                    let ta = transfer_active.clone();

                    tokio::spawn(handle_incoming(
                        stream, addr.ip().to_string(), dl, chunk_size, st, c, ir, ta, ctx2,
                    ));
                }
                Err(e) => {
                    tracing::warn!("Accept error: {e}");
                }
            }
        }
    });
}

async fn handle_incoming(
    stream: TcpStream,
    sender_ip: String,
    download_dir: PathBuf,
    chunk_size: usize,
    state: Arc<Mutex<TransferState>>,
    cancel: Arc<AtomicBool>,
    incoming_request: Arc<Mutex<Option<IncomingRequest>>>,
    transfer_active: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    tune_socket(&stream);

    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::with_capacity(IO_BUF_SIZE, read_half);
    let mut writer = BufWriter::with_capacity(4096, write_half);

    let request: TransferRequest = match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        read_message(&mut reader),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::warn!("Failed to read transfer request: {e}");
            let mut st = state.lock().unwrap();
            st.add_log(format!("Invalid transfer request from {sender_ip}: {e}"));
            ctx.request_repaint();
            return;
        }
        Err(_) => {
            tracing::warn!("Transfer request timeout from {sender_ip}");
            return;
        }
    };

    tracing::info!(
        "Transfer request from '{}' ({}): {} files, {} bytes",
        request.sender_name, sender_ip, request.file_count, request.total_size
    );

    let (tx, mut rx) = mpsc::channel::<bool>(1);
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);

    // Only one pending request at a time. If a decision is already on show,
    // decline this connection politely instead of clobbering the first one.
    let has_pending = {
        let mut ir = incoming_request.lock().unwrap();
        let busy = ir.is_some();
        if !busy {
            *ir = Some(IncomingRequest {
                sender_name: request.sender_name.clone(),
                sender_ip: sender_ip.clone(),
                files: request.files.clone(),
                total_size: request.total_size,
                request_id,
                decision: tx,
            });
        }
        busy
    };
    ctx.request_repaint();

    if has_pending {
        let response = TransferResponse { accepted: false };
        let _ = write_message(&mut writer, &response).await;
        let mut st = state.lock().unwrap();
        st.add_log(format!(
            "Declined transfer from '{}' — you already have a pending request.",
            request.sender_name
        ));
        ctx.request_repaint();
        return;
    }

    let accepted = match tokio::time::timeout(
        std::time::Duration::from_secs(120),
        rx.recv(),
    )
    .await
    {
        Ok(Some(decision)) => decision,
        _ => {
            tracing::info!("Transfer request timed out or superseded");
            {
                let mut ir = incoming_request.lock().unwrap();
                if ir.as_ref().map(|r| r.request_id) == Some(request_id) {
                    *ir = None;
                }
            }
            let response = TransferResponse { accepted: false };
            let _ = write_message(&mut writer, &response).await;
            ctx.request_repaint();
            return;
        }
    };

    {
        let mut ir = incoming_request.lock().unwrap();
        if ir.as_ref().map(|r| r.request_id) == Some(request_id) {
            *ir = None;
        }
    }
    ctx.request_repaint();

    // A single transfer (send or receive) at a time — the shared TransferState
    // only models one active transfer. Block politely if busy.
    if !transfer_active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        tracing::info!(
            "Rejecting '{}': another transfer is active",
            request.sender_name
        );
        let response = TransferResponse { accepted: false };
        let _ = write_message(&mut writer, &response).await;
        let mut st = state.lock().unwrap();
        st.add_log(format!(
            "Declined transfer from '{}' — another transfer is already running.",
            request.sender_name
        ));
        ctx.request_repaint();
        return;
    }
    let _guard = ActiveGuard(transfer_active.clone());

    let response = TransferResponse { accepted };
    if let Err(e) = write_message(&mut writer, &response).await {
        tracing::warn!("Failed to send response: {e}");
        return;
    }

    if !accepted {
        let mut st = state.lock().unwrap();
        st.add_log(format!("Declined transfer from {}", request.sender_name));
        ctx.request_repaint();
        return;
    }

    // Fresh start for this transfer: a stale cancel flag from a previous
    // (cancelled) transfer must not instantly cancel this new receive.
    cancel.store(false, Ordering::Relaxed);

    {
        let mut st = state.lock().unwrap();
        st.is_receiving = true;
        st.transferred_bytes = 0;
        st.total_bytes = request.total_size;
        st.transfers.clear();
        st.add_log(format!(
            "Accepted transfer from '{}'. Receiving {} files...",
            request.sender_name, request.file_count
        ));
    }
    ctx.request_repaint();

    let start_time = Instant::now();
    let mut total_received: u64 = 0;
    let mut file_index: usize = 0;

    loop {
        if cancel.load(Ordering::Relaxed) {
            let mut st = state.lock().unwrap();
            st.is_receiving = false;
            st.add_log("Receive cancelled".into());
            ctx.request_repaint();
            return;
        }

        let meta: FileMetadata = match read_message(&mut reader).await {
            Ok(m) => m,
            Err(e) => {
                let mut st = state.lock().unwrap();
                st.is_receiving = false;
                if total_received > 0 {
                    let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
                    let speed_mb = (total_received as f64 / elapsed) / (1024.0 * 1024.0);
                    st.add_log(format!(
                        "Receive complete! {:.1} MB in {:.1}s ({:.1} MB/s)",
                        total_received as f64 / (1024.0 * 1024.0),
                        elapsed,
                        speed_mb
                    ));
                } else {
                    st.add_log(format!("Receive ended: {e}"));
                }
                ctx.request_repaint();
                return;
            }
        };

        if meta.filename == END_MARKER_FILENAME {
            let mut st = state.lock().unwrap();
            st.is_receiving = false;
            let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
            let speed_mb = (total_received as f64 / elapsed) / (1024.0 * 1024.0);
            st.add_log(format!(
                "Receive complete! {:.1} MB in {:.1}s ({:.1} MB/s)",
                total_received as f64 / (1024.0 * 1024.0),
                elapsed,
                speed_mb
            ));
            ctx.request_repaint();
            return;
        }

        let out_path = if meta.relative_path.is_empty() {
            download_dir.join(&meta.filename)
        } else {
            download_dir.join(&meta.relative_path)
        };

        if let Some(parent) = out_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                tracing::warn!("Failed to create dir {}: {e}", parent.display());
            }
        }

        {
            let mut st = state.lock().unwrap();
            st.transfers.push(TransferInfo {
                filename: if meta.relative_path.is_empty() {
                    meta.filename.clone()
                } else {
                    meta.relative_path.clone()
                },
                total_size: meta.size,
                transferred: 0,
                status: TransferStatus::InProgress,
            });
        }
        ctx.request_repaint();

        let file = match tokio::fs::File::create(&out_path).await {
            Ok(f) => f,
            Err(e) => {
                let mut st = state.lock().unwrap();
                st.transfers[file_index].status = TransferStatus::Failed(e.to_string());
                st.add_log(format!("Failed to create {}: {e}", out_path.display()));
                st.is_receiving = false;
                ctx.request_repaint();
                return;
            }
        };

        let mut file_writer = BufWriter::with_capacity(IO_BUF_SIZE, file);
        let mut remaining = meta.size;
        let mut buf = vec![0u8; chunk_size];
        let mut last_ui_update = Instant::now();

        while remaining > 0 {
            if cancel.load(Ordering::Relaxed) {
                let mut st = state.lock().unwrap();
                st.is_receiving = false;
                st.transfers[file_index].status = TransferStatus::Cancelled;
                st.add_log("Receive cancelled".into());
                ctx.request_repaint();
                return;
            }

            let to_read = (remaining as usize).min(chunk_size);
            match reader.read_exact(&mut buf[..to_read]).await {
                Ok(_) => {}
                Err(e) => {
                    let mut st = state.lock().unwrap();
                    st.transfers[file_index].status = TransferStatus::Failed(e.to_string());
                    st.add_log(format!("Read error: {e}"));
                    st.is_receiving = false;
                    ctx.request_repaint();
                    return;
                }
            }

            if let Err(e) = file_writer.write_all(&buf[..to_read]).await {
                let mut st = state.lock().unwrap();
                st.transfers[file_index].status = TransferStatus::Failed(e.to_string());
                st.add_log(format!("Write error: {e}"));
                st.is_receiving = false;
                ctx.request_repaint();
                return;
            }

            remaining -= to_read as u64;
            total_received += to_read as u64;

            if last_ui_update.elapsed().as_millis() >= UI_UPDATE_INTERVAL_MS {
                let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
                {
                    let mut st = state.lock().unwrap();
                    st.transfers[file_index].transferred = meta.size - remaining;
                    st.transferred_bytes = total_received;
                    st.speed_bytes_per_sec = total_received as f64 / elapsed;
                }
                ctx.request_repaint();
                last_ui_update = Instant::now();
            }
        }

        if let Err(e) = file_writer.flush().await {
            tracing::warn!("Flush error for {}: {e}", out_path.display());
        }

        {
            let mut st = state.lock().unwrap();
            st.transfers[file_index].status = TransferStatus::Complete;
            st.transfers[file_index].transferred = meta.size;
            st.add_log(format!("Received: {}", if meta.relative_path.is_empty() { &meta.filename } else { &meta.relative_path }));
        }
        ctx.request_repaint();

        file_index += 1;
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
