use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Responder = oneshot::Sender<Result<Value>>;

/// CDP connection managing WebSocket communication
pub struct CdpConnection {
    command_tx: mpsc::UnboundedSender<(u32, String, Value, Responder)>,
    next_id: Arc<Mutex<u32>>,
}

impl CdpConnection {
    /// Connect to Chrome DevTools Protocol via WebSocket
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (command_tx, mut command_rx) =
            mpsc::unbounded_channel::<(u32, String, Value, Responder)>();
        let pending: Arc<Mutex<HashMap<u32, Responder>>> = Arc::new(Mutex::new(HashMap::new()));

        // Task for sending commands
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            while let Some((id, method, params, responder)) = command_rx.recv().await {
                let msg = json!({
                    "id": id,
                    "method": method,
                    "params": params
                });

                pending_clone.lock().await.insert(id, responder);

                if let Err(e) = write.send(Message::Text(msg.to_string().into())).await {
                    eprintln!("Failed to send CDP command: {}", e);
                    break;
                }
            }
        });

        // Task for receiving responses
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&text) {
                            // Handle response
                            if let Some(id) = v["id"].as_u64() {
                                let id = id as u32;
                                let responder = pending.lock().await.remove(&id);
                                if let Some(responder) = responder {
                                    if let Some(error) = v.get("error") {
                                        let _ = responder.send(Err(anyhow!(
                                            "CDP error: {}",
                                            error["message"].as_str().unwrap_or("unknown")
                                        )));
                                    } else if let Some(result) = v.get("result") {
                                        let _ = responder.send(Ok(result.clone()));
                                    }
                                }
                            }
                            // Ignore events for now (we'll handle them separately if needed)
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { command_tx, next_id: Arc::new(Mutex::new(1)) })
    }

    /// Send a CDP command and wait for response
    pub async fn send_command(&self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send((id, method.to_string(), params, tx))
            .map_err(|_| anyhow!("Failed to send command"))?;

        rx.await.map_err(|_| anyhow!("Response channel closed"))?
    }
}
