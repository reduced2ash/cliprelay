//! Telegram delivery: Bot API over blocking HTTP, and personal accounts
//! over MTProto (grammers) on a dedicated tokio thread.
//!
//! Ported from `telegram.py` — every user-visible string and every status
//! matches the original.

use crate::secrets::SecretStore;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const BOT_SESSION_KEY: &str = "telegram_bot_token";
pub const PERSONAL_SESSION_KEY: &str = "telegram_personal_session";
pub const PERSONAL_API_HASH_KEY: &str = "telegram_api_hash";

pub const BOT_MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramDelivery {
    pub message_id: String,
    pub link: String,
    pub detail: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    PasswordRequired(String),
}

fn telegram_error(message: impl Into<String>) -> TelegramError {
    TelegramError::Message(message.into())
}

// ---- Bot API --------------------------------------------------------------

pub struct TelegramBotService {
    secrets: SecretStore,
}

impl TelegramBotService {
    pub fn new(secrets: SecretStore) -> Self {
        Self { secrets }
    }

    pub fn has_token(&mut self) -> bool {
        !self.secrets.get(BOT_SESSION_KEY, "").is_empty()
    }

    fn client(&self, timeout: Option<Duration>) -> reqwest::blocking::Client {
        let mut builder = reqwest::blocking::Client::builder();
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        builder.build().unwrap_or_default()
    }

    /// Validate a bot token via `getMe`; stores it on success and returns
    /// the bot user JSON.
    pub fn validate(&mut self, token: &str) -> Result<serde_json::Value> {
        if token.trim().is_empty() || !token.contains(':') {
            return Err(telegram_error("Enter the complete bot token from @BotFather.").into());
        }
        let url = format!("https://api.telegram.org/bot{token}/getMe");
        let response = self
            .client(Some(Duration::from_secs(30)))
            .get(&url)
            .send()
            .map_err(|e| {
                telegram_error(format!("Telegram could not validate this bot token. ({e})"))
            })?;
        let payload: serde_json::Value = response.json().unwrap_or(serde_json::Value::Null);
        if !payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let description = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Telegram could not validate this bot token.");
            return Err(telegram_error(description).into());
        }
        self.secrets.set(BOT_SESSION_KEY, token)?;
        Ok(payload
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Resolve a destination via `getChat`; returns the chat object.
    pub fn validate_destination(&mut self, destination: &str) -> Result<serde_json::Value> {
        if !self.has_token() {
            return Err(telegram_error("Add a Telegram bot token first.").into());
        }
        if destination.trim().is_empty() {
            return Err(telegram_error("Enter a channel username or numeric chat ID.").into());
        }
        let token = self.secrets.get(BOT_SESSION_KEY, "");
        let url = format!("https://api.telegram.org/bot{token}/getChat");
        let response = self
            .client(Some(Duration::from_secs(30)))
            .post(&url)
            .form(&[("chat_id", destination)])
            .send()
            .map_err(|e| {
                telegram_error(format!("Telegram could not find that destination. ({e})"))
            })?;
        let payload: serde_json::Value = response.json().unwrap_or(serde_json::Value::Null);
        if !payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let description = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Telegram could not find that destination.");
            return Err(telegram_error(description).into());
        }
        Ok(payload
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Send a video through `sendVideo` (multipart, streamed, no timeout).
    pub fn send_video(
        &mut self,
        path: &Path,
        caption: &str,
        destination: &str,
        progress: &mut dyn FnMut(f64, &str),
    ) -> Result<TelegramDelivery> {
        if !self.has_token() {
            return Err(telegram_error("The Telegram bot is not configured.").into());
        }
        let target = path.to_path_buf();
        if !target.is_file() {
            return Err(telegram_error("The prepared video is no longer available.").into());
        }
        let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
        if size > BOT_MAX_FILE_BYTES {
            return Err(telegram_error(
                "This file exceeds the standard Telegram bot upload limit. Compress it or use Personal account.",
            )
            .into());
        }
        progress(0.05, "Uploading to Telegram");
        let token = self.secrets.get(BOT_SESSION_KEY, "");
        let url = format!("https://api.telegram.org/bot{token}/sendVideo");
        let file = std::fs::File::open(&target)?;
        let part = reqwest::blocking::multipart::Part::reader(file)
            .file_name(
                target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "video.mp4".into()),
            )
            .mime_str("video/mp4")?;
        let form = reqwest::blocking::multipart::Form::new()
            .text("chat_id", destination.to_string())
            .text("caption", caption.chars().take(1024).collect::<String>())
            .text("supports_streaming", "true")
            .part("video", part);
        let response = self
            .client(None)
            .post(&url)
            .multipart(form)
            .send()
            .map_err(|e| telegram_error(format!("Telegram could not send this video. ({e})")))?;
        let payload: serde_json::Value = response.json().unwrap_or(serde_json::Value::Null);
        if !payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            let description = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Telegram could not send this video.");
            return Err(telegram_error(description).into());
        }
        let message = payload
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let chat = message
            .get("chat")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let username = chat.get("username").and_then(|v| v.as_str()).unwrap_or("");
        let message_id = message
            .get("message_id")
            .map(|v| v.to_string())
            .unwrap_or_default();
        let link = if !username.is_empty() && !message_id.is_empty() {
            format!("https://t.me/{username}/{message_id}")
        } else {
            String::new()
        };
        progress(1.0, "Telegram sent");
        Ok(TelegramDelivery {
            message_id,
            link,
            detail: "Sent through bot".to_string(),
        })
    }
}

// ---- Personal (MTProto) ---------------------------------------------------

#[derive(Debug, Clone)]
pub struct DialogInfo {
    pub id: String,
    pub name: String,
    pub title: String,
    pub is_channel: bool,
    pub is_group: bool,
}

/// A `grammers_session::Session` backed by in-memory data with JSON
/// (de)serialization for persistence in the secret store. `SessionData`
/// itself has no serde support, so we round-trip through a mirror struct.
#[derive(Default)]
struct JsonSession(Mutex<grammers_client::session::SessionData>);

#[derive(serde::Serialize, serde::Deserialize)]
struct SessionSnapshot {
    home_dc: i32,
    dc_options: std::collections::HashMap<i32, grammers_client::session::types::DcOption>,
    peer_infos: std::collections::HashMap<
        grammers_client::session::types::PeerId,
        grammers_client::session::types::PeerInfo,
    >,
    updates_state: grammers_client::session::types::UpdatesState,
}

#[derive(Debug)]
struct JsonSessionError(String);

impl std::fmt::Display for JsonSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsonSessionError {}

impl JsonSession {
    fn from_json(json: &str) -> Option<Arc<Self>> {
        let snapshot: SessionSnapshot = serde_json::from_str(json).ok()?;
        let data = grammers_client::session::SessionData {
            home_dc: snapshot.home_dc,
            dc_options: snapshot.dc_options,
            peer_infos: snapshot.peer_infos,
            updates_state: snapshot.updates_state,
        };
        Some(Arc::new(Self(Mutex::new(data))))
    }

    fn to_json(&self) -> String {
        let data = self.0.lock();
        let snapshot = SessionSnapshot {
            home_dc: data.home_dc,
            dc_options: data.dc_options.clone(),
            peer_infos: data.peer_infos.clone(),
            updates_state: data.updates_state.clone(),
        };
        serde_json::to_string(&snapshot).unwrap_or_default()
    }
}

impl grammers_client::session::Session for JsonSession {
    type Error = JsonSessionError;

    fn home_dc_id(&self) -> Result<i32, Self::Error> {
        Ok(self.0.lock().home_dc)
    }

    fn set_home_dc_id(
        &self,
        dc_id: i32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send + '_>>
    {
        Box::pin(async move {
            self.0.lock().home_dc = dc_id;
            Ok(())
        })
    }

    fn dc_option(
        &self,
        dc_id: i32,
    ) -> Result<Option<grammers_client::session::types::DcOption>, Self::Error> {
        Ok(self.0.lock().dc_options.get(&dc_id).cloned())
    }

    fn set_dc_option(
        &self,
        dc_option: &grammers_client::session::types::DcOption,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send + '_>>
    {
        let dc_option = dc_option.clone();
        Box::pin(async move {
            self.0.lock().dc_options.insert(dc_option.id, dc_option);
            Ok(())
        })
    }

    fn peer(
        &self,
        peer: grammers_client::session::types::PeerId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<grammers_client::session::types::PeerInfo>, Self::Error>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move { Ok(self.0.lock().peer_infos.get(&peer).cloned()) })
    }

    fn cache_peer(
        &self,
        peer: &grammers_client::session::types::PeerInfo,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send + '_>>
    {
        let peer = peer.clone();
        Box::pin(async move {
            self.0
                .lock()
                .peer_infos
                .entry(peer.id())
                .or_insert_with(|| peer.clone())
                .extend_info(&peer);
            Ok(())
        })
    }

    fn updates_state(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<grammers_client::session::types::UpdatesState, Self::Error>,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move { Ok(self.0.lock().updates_state.clone()) })
    }

    fn set_update_state(
        &self,
        update: grammers_client::session::types::UpdateState,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send + '_>>
    {
        Box::pin(async move {
            let mut data = self.0.lock();
            match update {
                grammers_client::session::types::UpdateState::All(state) => {
                    data.updates_state = state
                }
                grammers_client::session::types::UpdateState::Primary { pts, date, seq } => {
                    data.updates_state.pts = pts;
                    data.updates_state.date = date;
                    data.updates_state.seq = seq;
                }
                grammers_client::session::types::UpdateState::Secondary { qts } => {
                    data.updates_state.qts = qts;
                }
                grammers_client::session::types::UpdateState::Channel { id: _, pts } => {
                    data.updates_state.pts = pts;
                }
            }
            Ok(())
        })
    }
}

type Reply<T> = std::sync::mpsc::Sender<Result<T>>;
pub type ProgressCb = Arc<dyn Fn(f64, &str) + Send + Sync>;

enum Command {
    Shutdown,
    BeginLogin {
        api_id: i32,
        api_hash: String,
        phone: String,
        reply: Reply<()>,
    },
    CompleteLogin {
        code: String,
        password: String,
        reply: Reply<(String, String)>,
    },
    Dialogs {
        api_id: i32,
        api_hash: String,
        session_json: String,
        reply: Reply<(Vec<DialogInfo>, String)>,
    },
    SendVideo {
        api_id: i32,
        api_hash: String,
        session_json: String,
        destination: String,
        path: PathBuf,
        caption: String,
        progress: ProgressCb,
        reply: Reply<(TelegramDelivery, String)>,
    },
    SignOut {
        api_id: i32,
        api_hash: String,
        session_json: String,
        reply: Reply<()>,
    },
}

struct LoopState {
    client: Option<grammers_client::Client>,
    session: Option<Arc<JsonSession>>,
    phone: String,
    api_id: i32,
    api_hash: String,
    token: Option<grammers_client::client::LoginToken>,
    password_token: Option<grammers_client::client::PasswordToken>,
    awaiting_password: bool,
}

impl LoopState {
    fn new() -> Self {
        Self {
            client: None,
            session: None,
            phone: String::new(),
            api_id: 0,
            api_hash: String::new(),
            token: None,
            password_token: None,
            awaiting_password: false,
        }
    }

    fn session_json(&self) -> String {
        self.session
            .as_ref()
            .map(|s| s.to_json())
            .unwrap_or_default()
    }

    fn connect(
        &mut self,
        session: Arc<JsonSession>,
        api_id: i32,
    ) -> Result<grammers_client::Client> {
        let pool = grammers_client::sender::SenderPool::new(session.clone(), api_id);
        let client = grammers_client::Client::new(pool.handle);
        self.session = Some(session);
        Ok(client)
    }
}

pub struct TelegramPersonalService {
    tx: std::sync::mpsc::Sender<Command>,
    handle: Option<std::thread::JoinHandle<()>>,
    exit_rx: Option<std::sync::mpsc::Receiver<()>>,
}

impl Default for TelegramPersonalService {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramPersonalService {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Command>();
        let (exit_tx, exit_rx) = std::sync::mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("tokio runtime");
            runtime.block_on(personal_loop(rx, exit_tx));
        });
        Self {
            tx,
            handle: Some(handle),
            exit_rx: Some(exit_rx),
        }
    }

    fn request<T>(
        &self,
        command: Command,
        reply_rx: std::sync::mpsc::Receiver<Result<T>>,
    ) -> Result<T> {
        self.tx
            .send(command)
            .map_err(|_| anyhow!("Telegram service is shutting down."))?;
        reply_rx
            .recv_timeout(Duration::from_secs(600))
            .map_err(|_| anyhow!("Telegram request timed out."))?
    }

    /// Begin the login flow: request a code for `phone`.
    pub fn begin_login(&self, api_id: i32, api_hash: &str, phone: &str) -> Result<()> {
        if api_id <= 0 || api_hash.trim().is_empty() || phone.trim().is_empty() {
            return Err(
                telegram_error("Enter your Telegram API ID, API hash, and phone number.").into(),
            );
        }
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.request(
            Command::BeginLogin {
                api_id,
                api_hash: api_hash.trim().to_string(),
                phone: phone.trim().to_string(),
                reply: reply_tx,
            },
            reply_rx,
        )
    }

    /// Complete login with the SMS code, or the 2FA password when required.
    /// Returns (display name, fresh session JSON).
    pub fn complete_login(&self, code: &str, password: &str) -> Result<(String, String)> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.request(
            Command::CompleteLogin {
                code: code.trim().to_string(),
                password: password.to_string(),
                reply: reply_tx,
            },
            reply_rx,
        )
    }

    pub fn dialogs(
        &self,
        api_id: i32,
        api_hash: &str,
        session_json: &str,
    ) -> Result<(Vec<DialogInfo>, String)> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.request(
            Command::Dialogs {
                api_id,
                api_hash: api_hash.to_string(),
                session_json: session_json.to_string(),
                reply: reply_tx,
            },
            reply_rx,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_video(
        &self,
        api_id: i32,
        api_hash: &str,
        session_json: &str,
        destination: &str,
        path: &Path,
        caption: &str,
        progress: ProgressCb,
    ) -> Result<(TelegramDelivery, String)> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.request(
            Command::SendVideo {
                api_id,
                api_hash: api_hash.to_string(),
                session_json: session_json.to_string(),
                destination: destination.to_string(),
                path: path.to_path_buf(),
                caption: caption.to_string(),
                progress,
                reply: reply_tx,
            },
            reply_rx,
        )
    }

    pub fn sign_out(&self, api_id: i32, api_hash: &str, session_json: &str) -> Result<()> {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.request(
            Command::SignOut {
                api_id,
                api_hash: api_hash.to_string(),
                session_json: session_json.to_string(),
                reply: reply_tx,
            },
            reply_rx,
        )
    }
}

impl Drop for TelegramPersonalService {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        // Bound the wait: an in-flight network command must not stall app
        // shutdown. Join only when the loop exited within the bound;
        // otherwise detach (the process is exiting anyway).
        let exited = self
            .exit_rx
            .take()
            .map(|exit_rx| exit_rx.recv_timeout(Duration::from_secs(5)).is_ok())
            .unwrap_or(false);
        if let Some(handle) = self.handle.take() {
            if exited {
                let _ = handle.join();
            }
        }
    }
}

async fn personal_loop(
    rx: std::sync::mpsc::Receiver<Command>,
    exit_tx: std::sync::mpsc::Sender<()>,
) {
    let (tokio_tx, mut tokio_rx) = tokio::sync::mpsc::channel::<Command>(64);
    tokio::spawn(async move {
        loop {
            match rx.try_recv() {
                Ok(command) => {
                    if tokio_tx.send(command).await.is_err() {
                        break;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
    });
    let mut state = LoopState::new();
    while let Some(command) = tokio_rx.recv().await {
        match command {
            Command::BeginLogin {
                api_id,
                api_hash,
                phone,
                reply,
            } => {
                let result = async {
                    let session = Arc::new(JsonSession::default());
                    let client = state.connect(session, api_id)?;
                    let token =
                        client
                            .request_login_code(&phone, &api_hash)
                            .await
                            .map_err(|e| {
                                telegram_error(format!(
                                    "Telegram could not send the sign-in code. ({e})"
                                ))
                            })?;
                    state.client = Some(client);
                    state.phone = phone.clone();
                    state.api_id = api_id;
                    state.api_hash = api_hash;
                    state.token = Some(token);
                    state.password_token = None;
                    state.awaiting_password = false;
                    Ok(())
                }
                .await;
                let _ = reply.send(result);
            }
            Command::CompleteLogin {
                code,
                password,
                reply,
            } => {
                let result = async {
                    let Some(client) = state.client.as_ref() else {
                        return Err(telegram_error("Request a Telegram sign-in code first.").into());
                    };
                    if state.awaiting_password {
                        if password.trim().is_empty() {
                            return Err(TelegramError::PasswordRequired(
                                "Enter this account's two-step verification password.".into(),
                            )
                            .into());
                        }
                        // Keep the token on failure so a wrong password can be
                        // retried (mirrors the original's 2FA flow): grammers
                        // returns a fresh token inside InvalidPassword.
                        let Some(password_token) = state.password_token.take() else {
                            return Err(
                                telegram_error("Request a Telegram sign-in code first.").into()
                            );
                        };
                        match client
                            .check_password(password_token, password.as_bytes())
                            .await
                        {
                            Ok(user) => {
                                state.awaiting_password = false;
                                return Ok((display_name(&user), state.session_json()));
                            }
                            Err(grammers_client::client::SignInError::InvalidPassword(token)) => {
                                state.password_token = Some(token);
                                return Err(TelegramError::PasswordRequired(
                                    "The two-step verification password was incorrect. Try again."
                                        .into(),
                                )
                                .into());
                            }
                            Err(e) => {
                                return Err(telegram_error(format!(
                                    "Telegram sign-in did not complete. ({e})"
                                ))
                                .into())
                            }
                        }
                    }
                    // Keep the code token on failure so a mistyped code can be
                    // retried without requesting a new one. A PasswordRequired
                    // result consumes it: the code was correct and the flow
                    // moves to the password step.
                    let Some(token) = state.token.take() else {
                        return Err(telegram_error("Request a Telegram sign-in code first.").into());
                    };
                    match client.sign_in(&token, &code).await {
                        Ok(user) => Ok((display_name(&user), state.session_json())),
                        Err(grammers_client::client::SignInError::PasswordRequired(token)) => {
                            state.password_token = Some(token);
                            state.awaiting_password = true;
                            Err(TelegramError::PasswordRequired(
                                "This Telegram account requires its two-step verification password."
                                    .into(),
                            )
                            .into())
                        }
                        Err(e) => {
                            state.token = Some(token);
                            Err(
                                telegram_error(format!("Telegram sign-in did not complete. ({e})"))
                                    .into(),
                            )
                        }
                    }
                }
                .await;
                let _ = reply.send(result);
            }
            Command::Dialogs {
                api_id,
                api_hash,
                session_json,
                reply,
            } => {
                let result = dialogs_impl(&mut state, api_id, &api_hash, &session_json).await;
                let _ = reply.send(result);
            }
            Command::SendVideo {
                api_id,
                api_hash,
                session_json,
                destination,
                path,
                caption,
                progress,
                reply,
            } => {
                let result = send_video_impl(
                    &mut state,
                    api_id,
                    &api_hash,
                    &session_json,
                    &destination,
                    &path,
                    &caption,
                    progress,
                )
                .await;
                let _ = reply.send(result);
            }
            Command::SignOut {
                api_id,
                api_hash,
                session_json,
                reply,
            } => {
                let result = sign_out_impl(&mut state, api_id, &api_hash, &session_json).await;
                let _ = reply.send(result);
            }
            // Dedicated shutdown (service drop): exit the loop so the worker
            // thread can be joined without blocking app exit.
            Command::Shutdown => {
                break;
            }
        }
    }
    let _ = exit_tx.send(());
}

fn display_name(user: &grammers_client::peer::User) -> String {
    let first = user.first_name().unwrap_or("").trim().to_string();
    let last = user.last_name().unwrap_or("").trim().to_string();
    let joined = [first, last]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !joined.is_empty() {
        joined
    } else {
        user.username().unwrap_or("Telegram account").to_string()
    }
}

fn require_session(session_json: &str, api_id: i32, api_hash: &str) -> Result<Arc<JsonSession>> {
    if session_json.trim().is_empty() || api_id <= 0 || api_hash.trim().is_empty() {
        return Err(telegram_error("The personal Telegram account is not signed in.").into());
    }
    JsonSession::from_json(session_json)
        .ok_or_else(|| telegram_error("The personal Telegram account is not signed in.").into())
}

async fn connect_authorized(
    state: &mut LoopState,
    api_id: i32,
    api_hash: &str,
    session_json: &str,
) -> Result<grammers_client::Client> {
    let session = require_session(session_json, api_id, api_hash)?;
    let client = state.connect(session, api_id)?;
    let authorized = client.is_authorized().await.map_err(|e| {
        telegram_error(format!(
            "The personal Telegram account is not signed in. ({e})"
        ))
    })?;
    if !authorized {
        return Err(
            telegram_error("The Telegram session expired. Sign in again in Settings.").into(),
        );
    }
    Ok(client)
}

async fn dialogs_impl(
    state: &mut LoopState,
    api_id: i32,
    api_hash: &str,
    session_json: &str,
) -> Result<(Vec<DialogInfo>, String)> {
    let client = connect_authorized(state, api_id, api_hash, session_json).await?;
    let mut dialogs = client.iter_dialogs();
    let mut out = Vec::new();
    for _ in 0..250 {
        let Some(dialog) = dialogs
            .next()
            .await
            .map_err(|e| telegram_error(format!("Telegram could not load chats. ({e})")))?
        else {
            break;
        };
        let peer = dialog.peer();
        let is_bot = matches!(peer, grammers_client::peer::Peer::User(user) if user.is_bot());
        if is_bot {
            continue;
        }
        let id = peer.id().to_string();
        let name = peer.name().unwrap_or(&id).to_string();
        let is_channel = matches!(peer, grammers_client::peer::Peer::Channel(_));
        let is_group = matches!(peer, grammers_client::peer::Peer::Group(_));
        out.push(DialogInfo {
            id: id.clone(),
            name: name.clone(),
            title: name,
            is_channel,
            is_group,
        });
    }
    Ok((out, state.session_json()))
}

#[allow(clippy::too_many_arguments)]
async fn send_video_impl(
    state: &mut LoopState,
    api_id: i32,
    api_hash: &str,
    session_json: &str,
    destination: &str,
    path: &Path,
    caption: &str,
    progress: ProgressCb,
) -> Result<(TelegramDelivery, String)> {
    let client = connect_authorized(state, api_id, api_hash, session_json).await?;
    if !path.is_file() {
        return Err(telegram_error("The prepared video is no longer available.").into());
    }
    let destination = destination.trim();
    if destination.is_empty() {
        return Err(telegram_error("Enter a channel username or numeric chat ID.").into());
    }
    // Resolve the destination: numeric IDs become peers directly, anything
    // else is a username (mirrors Telethon's `get_input_entity`).
    let peer_ref = if let Some(numeric) = parse_numeric(destination) {
        let id = if numeric < 0 {
            grammers_client::session::types::PeerId::chat_unchecked(-numeric)
        } else {
            grammers_client::session::types::PeerId::user_unchecked(numeric)
        };
        id.to_ambient_ref()
    } else {
        let Some(peer) = client
            .resolve_username(destination.trim_start_matches('@'))
            .await
            .map_err(|e| {
                telegram_error(format!("Telegram could not find that destination. ({e})"))
            })?
        else {
            return Err(telegram_error("Telegram could not find that destination.").into());
        };
        let Some(peer_ref) = peer.to_ref().await.map_err(|e| {
            telegram_error(format!("Telegram could not find that destination. ({e})"))
        })?
        else {
            return Err(telegram_error("Telegram could not find that destination.").into());
        };
        peer_ref
    };
    let link_username = if parse_numeric(destination).is_some() {
        None
    } else {
        client
            .resolve_username(destination.trim_start_matches('@'))
            .await
            .ok()
            .flatten()
            .and_then(|peer| peer.username().map(|u| u.to_string()))
    };

    progress(0.05, "Uploading to Telegram");
    let size = std::fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);
    let file = tokio::fs::File::open(path).await?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video.mp4".to_string());
    let done = Arc::new(AtomicUsize::new(0));
    let mut reader = ProgressReader {
        inner: file,
        done: Arc::clone(&done),
        total: size.max(1),
        callback: Arc::clone(&progress),
    };
    let uploaded = client
        .upload_stream(&mut reader, size, name.clone())
        .await
        .map_err(|e| telegram_error(format!("Telegram could not send this video. ({e})")))?;
    progress(0.9, "Uploading to Telegram");

    let (duration_secs, width, height) = crate::media::quick_probe_info(path);
    let video_attribute = grammers_client::tl::enums::DocumentAttribute::Video(
        grammers_client::tl::types::DocumentAttributeVideo {
            round_message: false,
            supports_streaming: true,
            nosound: false,
            duration: (duration_secs as f64) / 1000.0,
            w: width.max(0) as i32,
            h: height.max(0) as i32,
            preload_prefix_size: None,
            video_start_ts: None,
            video_codec: None,
        },
    );
    let filename_attribute = grammers_client::tl::enums::DocumentAttribute::Filename(
        grammers_client::tl::types::DocumentAttributeFilename { file_name: name },
    );
    let raw_media: grammers_client::tl::enums::InputMedia =
        grammers_client::tl::types::InputMediaUploadedDocument {
            nosound_video: false,
            force_file: false,
            spoiler: false,
            file: uploaded.raw,
            thumb: None,
            mime_type: "video/mp4".to_string(),
            attributes: vec![video_attribute, filename_attribute],
            stickers: None,
            ttl_seconds: None,
            video_cover: None,
            video_timestamp: None,
        }
        .into();
    let message = client
        .send_message(
            peer_ref,
            grammers_client::message::InputMessage::new()
                .text(caption.chars().take(4096).collect::<String>())
                .media(raw_media),
        )
        .await
        .map_err(|e| telegram_error(format!("Telegram could not send this video. ({e})")))?;
    let message_id = message.id().to_string();
    let link = match link_username {
        Some(username) if !message_id.is_empty() => {
            format!("https://t.me/{username}/{message_id}")
        }
        _ => String::new(),
    };
    progress(1.0, "Telegram sent");
    Ok((
        TelegramDelivery {
            message_id,
            link,
            detail: "Sent through personal account".to_string(),
        },
        state.session_json(),
    ))
}

async fn sign_out_impl(
    state: &mut LoopState,
    api_id: i32,
    api_hash: &str,
    session_json: &str,
) -> Result<()> {
    if !session_json.trim().is_empty() && api_id > 0 && !api_hash.trim().is_empty() {
        if let Some(session) = JsonSession::from_json(session_json) {
            if let Ok(client) = state.connect(session, api_id) {
                let _ = client.sign_out().await;
            }
        }
    }
    state.client = None;
    state.session = None;
    state.token = None;
    state.password_token = None;
    state.awaiting_password = false;
    Ok(())
}

fn parse_numeric(value: &str) -> Option<i64> {
    let digits = value.trim_start_matches('-');
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits
        .parse::<i64>()
        .ok()
        .map(|n| if value.starts_with('-') { -n } else { n })
}

/// Counting reader that reports upload progress.
struct ProgressReader<R> {
    inner: R,
    done: Arc<AtomicUsize>,
    total: usize,
    callback: ProgressCb,
}

impl<R: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        if let std::task::Poll::Ready(Ok(())) = &result {
            let read = buf.filled().len() - before;
            if read > 0 {
                let done = self.done.fetch_add(read, Ordering::Relaxed) + read;
                (self.callback)(
                    (done as f64 / self.total as f64).min(0.9),
                    "Uploading to Telegram",
                );
            }
        }
        result
    }
}

/// Personal-service convenience wrapper bound to the secret store.
pub struct PersonalTelegram {
    pub service: TelegramPersonalService,
    pub secrets: SecretStore,
}

impl PersonalTelegram {
    pub fn new(secrets: SecretStore) -> Self {
        Self {
            service: TelegramPersonalService::new(),
            secrets,
        }
    }

    fn api_hash(&mut self) -> Result<String> {
        let api_hash = self.secrets.get(PERSONAL_API_HASH_KEY, "");
        if api_hash.is_empty() {
            return Err(telegram_error("The personal Telegram account is not signed in.").into());
        }
        Ok(api_hash)
    }

    pub fn begin_login(&self, api_id: i32, api_hash: &str, phone: &str) -> Result<()> {
        // Persist the hash immediately so dialogs/delivery work even if the
        // app restarts mid-login (mirrors the original storing it in
        // settings at the same point).
        self.secrets.set(PERSONAL_API_HASH_KEY, api_hash)?;
        self.service.begin_login(api_id, api_hash, phone)
    }

    pub fn complete_login(&mut self, code: &str, password: &str) -> Result<String> {
        let (display, session_json) = self.service.complete_login(code, password)?;
        self.secrets.set(PERSONAL_SESSION_KEY, &session_json)?;
        Ok(display)
    }

    pub fn dialogs(&mut self, api_id: i32) -> Result<Vec<DialogInfo>> {
        let api_hash = self.api_hash()?;
        let session_json = self.secrets.get(PERSONAL_SESSION_KEY, "");
        let (dialogs, session_json) = self.service.dialogs(api_id, &api_hash, &session_json)?;
        self.secrets.set(PERSONAL_SESSION_KEY, &session_json)?;
        Ok(dialogs)
    }

    pub fn send_video(
        &mut self,
        api_id: i32,
        destination: &str,
        path: &Path,
        caption: &str,
        progress: ProgressCb,
    ) -> Result<TelegramDelivery> {
        let api_hash = self.api_hash()?;
        let session_json = self.secrets.get(PERSONAL_SESSION_KEY, "");
        let (delivery, session_json) = self.service.send_video(
            api_id,
            &api_hash,
            &session_json,
            destination,
            path,
            caption,
            progress,
        )?;
        self.secrets.set(PERSONAL_SESSION_KEY, &session_json)?;
        Ok(delivery)
    }

    pub fn sign_out(&mut self, api_id: i32) {
        if api_id > 0 {
            if let Ok(api_hash) = self.api_hash() {
                let session_json = self.secrets.get(PERSONAL_SESSION_KEY, "");
                let _ = self.service.sign_out(api_id, &api_hash, &session_json);
            }
        }
        self.secrets.delete(PERSONAL_SESSION_KEY);
        self.secrets.delete(PERSONAL_API_HASH_KEY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::test_util::KEYCHAIN_LOCK;

    #[test]
    fn numeric_parsing() {
        assert_eq!(parse_numeric("12345"), Some(12345));
        assert_eq!(parse_numeric("-100123"), Some(-100123));
        assert_eq!(parse_numeric("@channel"), None);
        assert_eq!(parse_numeric(""), None);
        assert_eq!(parse_numeric("12a"), None);
    }

    #[test]
    fn token_format_validation() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = SecretStore::new(Some(dir.path().to_path_buf()));
        let mut bot = TelegramBotService::new(secrets);
        // Missing colon -> specific error before any network call.
        let err = bot.validate("12345").unwrap_err();
        assert!(err.to_string().contains("@BotFather"));
        let err = bot.validate("").unwrap_err();
        assert!(err.to_string().contains("@BotFather"));
    }

    #[test]
    fn service_survives_user_sign_out() {
        let _guard = KEYCHAIN_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let secrets = SecretStore::new(Some(dir.path().to_path_buf()));
        secrets.force_fallback_for_tests();
        let personal = PersonalTelegram::new(secrets);
        // A user-initiated sign-out (no session) must not kill the service.
        let _ = personal.service.sign_out(0, "", "");
        // The service must still answer subsequent requests.
        let err = personal.begin_login(0, "", "").unwrap_err();
        assert!(
            err.to_string().contains("Enter your Telegram API ID"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn api_hash_reads_persisted_value() {
        let _guard = KEYCHAIN_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let secrets = SecretStore::new(Some(dir.path().to_path_buf()));
        secrets.force_fallback_for_tests();
        let hash = "0123456789abcdef0123456789abcdef";
        secrets.set(PERSONAL_API_HASH_KEY, hash).unwrap();
        let mut personal = PersonalTelegram::new(secrets);
        assert_eq!(personal.api_hash().unwrap(), hash);
        // Deleting the key surfaces the sign-in error instead of panicking
        // (also cleans the credential from the OS keychain).
        personal.secrets.delete(PERSONAL_API_HASH_KEY);
        let err = personal.api_hash().unwrap_err();
        assert!(err.to_string().contains("not signed in"));
    }
}
