use std::{convert::Infallible, process::Stdio, str::FromStr};

use futures_util::{stream, SinkExt, StreamExt};
use tokio::{fs, process::Command};
use url::Url;
use warp::{Filter, Rejection, Reply};

use crate::{config::Config, lsp};

use super::with_context;

#[derive(Debug, Clone)]
pub struct Context {
    /// One or more commands to start a Language Server.
    /// If not specified, the first one is started.
    /// Maybe use `Option<Vec<Vec<String>>>` to allow no commands.
    pub commands: Option<Vec<Vec<String>>>,
    /// Write file on save.
    pub sync: bool,
    /// Remap relative `source://` to absolute `file://`.
    pub remap: bool,
    /// Project root.
    pub cwd: Url,
    /// config
    pub config: Option<Config>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct Query {
    /// The command name of the Language Server to start.
    /// If not specified, the first one is started.
    name: String,
}

fn with_optional_query() -> impl Filter<Extract = (Option<Query>,), Error = Infallible> + Clone {
    warp::query::<Query>()
        .map(Some)
        .or_else(|_| async { Ok::<(Option<Query>,), Infallible>((None,)) })
}

/// Handler for WebSocket connection.
pub fn handler(ctx: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::end()
        .and(warp::ws())
        .and(with_context(ctx))
        .and(with_optional_query())
        .map(|ws: warp::ws::Ws, ctx, query| {
            ws.with_compression()
                .on_upgrade(move |socket| on_upgrade(socket, ctx, query))
        })
}

#[tracing::instrument(level = "debug", err, skip(msg))]
async fn maybe_write_text_document(msg: &lsp::Message) -> Result<(), std::io::Error> {
    if let lsp::Message::Notification(lsp::Notification::DidSave { params }) = msg {
        if let Some(text) = &params.text {
            let uri = &params.text_document.uri;
            if uri.scheme() == "file" {
                if let Ok(path) = uri.to_file_path() {
                    if let Some(parent) = path.parent() {
                        tracing::debug!("writing to {:?}", path);
                        fs::create_dir_all(parent).await?;
                        fs::write(&path, text.as_bytes()).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn on_upgrade(socket: warp::ws::WebSocket, ctx: Context, query: Option<Query>) {
    tracing::info!("connected");
    if let Err(err) = connected(socket, ctx, query).await {
        tracing::error!("connection error: {}", err);
    }
    tracing::info!("disconnected");
}

fn get_command<'a>(ctx: &'a Context, query: &'a Option<Query>) -> Option<&'a Vec<String>> {
    if let Some(query) = query {
        if let Some(config) = &ctx.config {
            if let Some(servers) = &config.servers {
                if let Some(sc) = servers.get(&query.name) {
                    return Some(&sc.command);
                }
            }
        }
        if let Some(command) = ctx
            .commands
            .as_ref()
            .and_then(|c| c.iter().find(|v| v[0] == query.name))
        {
            Some(command)
        } else {
            let not_found_error = &ctx.config.as_ref().map_or(false, |c| c.not_found_error);
            if *not_found_error {
                None
            } else {
                tracing::warn!("no command found for {:?}, using the first one", query);
                ctx.commands.as_ref().and_then(|c| c.first())
            }
        }
    } else {
        ctx.commands.as_ref().and_then(|c| c.first())
    }
}

#[tracing::instrument(level = "debug", skip(ws, ctx), fields(remap = %ctx.remap, sync = %ctx.sync))]
async fn connected(
    ws: warp::ws::WebSocket,
    ctx: Context,
    query: Option<Query>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let command =
        get_command(&ctx, &query).ok_or_else(|| format!("no command found for {:?}", query))?;
    tracing::info!("starting {} in {}", command[0], ctx.cwd);
    let mut server = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    tracing::debug!("running {}", command[0]);

    let mut server_send = lsp::framed::writer(server.stdin.take().unwrap());
    let mut server_recv = lsp::framed::reader(server.stdout.take().unwrap());
    let (mut client_send, client_recv) = ws.split();
    let client_recv = client_recv
        .filter_map(filter_map_warp_ws_message)
        // Chain this with `Done` so we know when the client disconnects
        .chain(stream::once(async { Ok(Message::Done) }));
    // Tick every 30s so we can ping the client to keep the connection alive
    let ticks = stream::unfold(
        tokio::time::interval(std::time::Duration::from_secs(30)),
        |mut interval| async move {
            interval.tick().await;
            Some((Ok(Message::Tick), interval))
        },
    );
    let mut client_recv = stream::select(client_recv, ticks).boxed();

    // let mut client_msg = client_recv.next();
    // let mut server_msg = server_recv.next();
    // Keeps track if `pong` was received since sending the last `ping`.
    let mut is_alive = true;

    let mut database = None;
    loop {
        tokio::select! {
            from_client = client_recv.next() => {
                match from_client {
                    // Valid LSP message
                    Some(Ok(Message::Message(mut msg))) => {
                        if ctx.remap {
                            lsp::ext::remap_relative_uri(&mut msg, &ctx.cwd)?;
                            tracing::debug!("remapped relative URI from client");
                        }
                        if ctx.sync {
                            maybe_write_text_document(&msg).await?;
                        }

                        database = lsp::ext::create_database_on_init(
                            &mut msg,
                            "sql",
                            ctx.config.as_ref(),
                        ).await?;
                        let text = serde_json::to_string(&msg)?;
                        tracing::debug!("-> {}", text);
                        server_send.send(text).await?;
                    }

                    // Invalid JSON body
                    Some(Ok(Message::Invalid(text))) => {
                        tracing::warn!("-> {}", text);
                        // Just forward it to the server as is.
                        server_send.send(text).await?;
                    }

                    // Close message
                    Some(Ok(Message::Close)) => {
                        // The connection will terminate when None is received.
                        tracing::info!("received Close message");
                    }

                    // Ping the client to keep the connection alive
                    Some(Ok(Message::Tick)) => {
                        // Terminate if we haven't heard back from the previous ping.
                        if !is_alive {
                            tracing::warn!("terminating unhealthy connection");
                            break;
                        }

                        is_alive = false;
                        tracing::debug!("pinging the client");
                        client_send.send(warp::ws::Message::ping(vec![])).await?;
                    }

                    // Mark the connection as alive on any pong.
                    Some(Ok(Message::Pong)) => {
                        tracing::debug!("received pong");
                        is_alive = true;
                    }

                    // Connection closed
                    Some(Ok(Message::Done)) => {
                        tracing::info!("connection closed");
                        break;
                    }

                    // WebSocket Error
                    Some(Err(err)) => {
                        tracing::error!("websocket error: {}", err);
                    }

                    None => {
                        // Unreachable because of the interval stream
                        unreachable!("should never yield None");
                    }
                }
            }
            from_server = server_recv.next() => {
                match from_server {
                    // Serialized LSP Message
                    Some(Ok(text)) => {
                        if ctx.remap {
                            if let Ok(mut msg) = lsp::Message::from_str(&text) {
                                lsp::ext::remap_relative_uri(&mut msg, &ctx.cwd)?;
                                tracing::debug!("remapped relative URI from server");
                                let text = serde_json::to_string(&msg)?;
                                tracing::debug!("<- {}", text);
                                client_send.send(warp::ws::Message::text(text)).await?;
                            } else {
                                tracing::warn!("<- {}", text);
                                client_send.send(warp::ws::Message::text(text)).await?;
                            }
                        } else {
                            tracing::debug!("<- {}", text);
                            client_send.send(warp::ws::Message::text(text)).await?;
                        }
                    }

                    // Codec Error
                    Some(Err(err)) => {
                        tracing::error!("{}", err);
                    }

                    // Server exited
                    None => {
                        tracing::error!("server process exited unexpectedly");
                        client_send.send(warp::ws::Message::close()).await?;
                        break;
                    }
                }
            }
        }
    }

    if let Some(mut database) = database {
        tracing::info!("drop database: {}", database.id());
        database.cleanup().await?;
    }

    Ok(())
}

// Type to describe a message from the client conveniently.
#[allow(clippy::large_enum_variant)]
#[allow(clippy::enum_variant_names)]
enum Message {
    // Valid LSP message
    Message(lsp::Message),
    // Invalid JSON
    Invalid(String),
    // Close message
    Close,
    // Ping the client to keep the connection alive.
    // Note that this is from the interval stream and not actually from client.
    Tick,
    // Client disconnected. Necessary because the combined stream is infinite.
    Done,
    // A reply for ping or heartbeat from client.
    Pong,
}

// Parse the message and ignore anything we don't care.
async fn filter_map_warp_ws_message(
    wsm: Result<warp::ws::Message, warp::Error>,
) -> Option<Result<Message, warp::Error>> {
    match wsm {
        Ok(msg) => {
            if msg.is_close() {
                Some(Ok(Message::Close))
            } else if msg.is_text() {
                let text = msg.to_str().expect("text");
                match lsp::Message::from_str(text) {
                    Ok(msg) => Some(Ok(Message::Message(msg))),
                    Err(_) => Some(Ok(Message::Invalid(text.to_owned()))),
                }
            } else if msg.is_pong() {
                Some(Ok(Message::Pong))
            } else {
                // Ignore any other message types
                None
            }
        }

        Err(err) => Some(Err(err)),
    }
}
