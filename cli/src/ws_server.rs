use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use miai::{PlayState, Xiaoai};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type ClientSender = futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>;
type Clients = Arc<RwLock<Vec<Arc<Mutex<ClientSender>>>>>;

/// WebSocket API 请求
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum ApiRequest {
    Say {
        device_id: String,
        text: String,
    },
    Play {
        device_id: String,
        url: Option<String>,
    },
    Pause {
        device_id: String,
    },
    Stop {
        device_id: String,
    },
    Volume {
        device_id: String,
        volume: u32,
    },
    Ask {
        device_id: String,
        text: String,
    },
    Status {
        device_id: String,
    },
    GetDevices,
}

/// WebSocket API 响应
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiResponse {
    Success {
        code: i64,
        message: String,
        data: serde_json::Value,
    },
    Error {
        error: String,
    },
    Devices {
        devices: Vec<DeviceData>,
    },
    KeywordMatch {
        timestamp: i64,
        query: String,
        matched_keyword: String,
        device_id: String,
    },
}

#[derive(Debug, Serialize)]
struct DeviceData {
    device_id: String,
    name: String,
    hardware: String,
}

/// WebSocket 服务器
#[derive(Clone)]
pub struct WsServer {
    xiaoai: Arc<Xiaoai>,
    port: u16,
    clients: Clients,
}

impl WsServer {
    pub fn new(xiaoai: Xiaoai, port: u16) -> Self {
        Self {
            xiaoai: Arc::new(xiaoai),
            port,
            clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn run_server(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(&addr).await?;
        
        eprintln!("🚀 WebSocket 服务器已启动");
        eprintln!("监听地址: ws://{}", addr);
        eprintln!("按 Ctrl+C 停止服务\n");

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let xiaoai = Arc::clone(&self.xiaoai);
            let clients = Arc::clone(&self.clients);
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer_addr, xiaoai, clients).await {
                    eprintln!("处理连接 {} 时出错: {}", peer_addr, e);
                }
            });
        }
    }

    /// 运行关键词监听器
    pub async fn run_watcher(&self, device_id: String, hardware: String) -> Result<()> {
        self.start_keyword_watcher(device_id, hardware).await
    }

    /// 启动关键词监听（内部方法）
    async fn start_keyword_watcher(&self, device_id: String, hardware: String) -> Result<()> {
        use miai::ConversationWatcher;
        
        let config_path = std::path::PathBuf::from("config.json");
        let mut watcher = ConversationWatcher::from_json_file(&config_path)
            .context("加载配置文件失败")?;
        
        let clients = Arc::clone(&self.clients);
        let xiaoai = Arc::clone(&self.xiaoai);
        
        eprintln!("🎧 开始监听关键词...");
        eprintln!("设备 ID: {}", device_id);
        eprintln!("设备型号: {}", hardware);
        
        let enabled_keywords: Vec<_> = watcher.get_enabled_keywords().collect();
        if enabled_keywords.is_empty() {
            eprintln!("⚠️  警告: 配置文件中没有启用的关键词");
        } else {
            eprintln!("📝 已启用的关键词:");
            for (i, kw) in enabled_keywords.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, kw);
            }
        }
        eprintln!("---\n");
        
        let device_id_clone = device_id.clone();
        
        watcher
            .watch(&xiaoai, &device_id, &hardware, move |keyword_match| {
                let device_id = device_id_clone.clone();
                let clients = Arc::clone(&clients);
                
                async move {
                    let response = ApiResponse::KeywordMatch {
                        timestamp: keyword_match.conversation.time,
                        query: keyword_match.conversation.query.clone(),
                        matched_keyword: keyword_match.matched_keyword.to_string(),
                        device_id,
                    };
                    
                    match serde_json::to_string(&response) {
                        Ok(response_text) => {
                            broadcast_message(&clients, response_text).await;
                        }
                        Err(e) => {
                            eprintln!("序列化响应失败: {}", e);
                        }
                    }
                    
                    Ok(())
                }
            })
            .await?;
        
        Ok(())
    }
}

/// 向所有连接的客户端广播消息
async fn broadcast_message(clients: &Clients, message: String) {
    let clients_lock = clients.read().await;
    let mut disconnected = Vec::new();
    
    for (idx, client) in clients_lock.iter().enumerate() {
        let mut sender = client.lock().await;
        if let Err(e) = sender.send(Message::Text(message.clone())).await {
            eprintln!("发送消息到客户端 {} 失败: {}", idx, e);
            disconnected.push(idx);
        }
    }
    
    drop(clients_lock);
    
    // 清理断开连接的客户端
    if !disconnected.is_empty() {
        let mut clients_lock = clients.write().await;
        for idx in disconnected.iter().rev() {
            clients_lock.remove(*idx);
            eprintln!("移除断开的客户端 {}", idx);
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    xiaoai: Arc<Xiaoai>,
    clients: Clients,
) -> Result<()> {
    eprintln!("✅ 新连接: {}", peer_addr);
    
    let ws_stream = accept_async(stream)
        .await
        .context("WebSocket 握手失败")?;
    
    let (ws_sender, mut ws_receiver) = ws_stream.split();
    
    let ws_sender = Arc::new(Mutex::new(ws_sender));
    
    // 将新客户端添加到客户端列表
    {
        let mut clients_lock = clients.write().await;
        clients_lock.push(Arc::clone(&ws_sender));
        eprintln!("当前连接数: {}", clients_lock.len());
    }
    
    while let Some(msg) = ws_receiver.next().await {
        let msg = msg?;
        
        if msg.is_close() {
            eprintln!("❌ 连接关闭: {}", peer_addr);
            break;
        }
        
        if !msg.is_text() {
            continue;
        }
        
        let text = msg.to_text()?;
        eprintln!("📨 收到消息: {}", text);
        
        let response = match serde_json::from_str::<ApiRequest>(text) {
            Ok(request) => {
                let ws_sender_clone = Arc::clone(&ws_sender);
                handle_request(request, &xiaoai, ws_sender_clone).await
            }
            Err(e) => ApiResponse::Error {
                error: format!("无效的请求格式: {}", e),
            },
        };
        
        let response_text = serde_json::to_string(&response)?;
        eprintln!("📤 发送响应: {}", response_text);
        
        let mut sender = ws_sender.lock().await;
        sender.send(Message::Text(response_text)).await?;
    }
    
    // 从客户端列表中移除
    {
        let mut clients_lock = clients.write().await;
        clients_lock.retain(|client| !Arc::ptr_eq(client, &ws_sender));
        eprintln!("当前连接数: {}", clients_lock.len());
    }
    
    Ok(())
}

async fn handle_request(
    request: ApiRequest,
    xiaoai: &Xiaoai,
    _ws_sender: Arc<Mutex<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>>>,
) -> ApiResponse {
    let result = match request {
        ApiRequest::Say { device_id, text } => {
            xiaoai.tts(&device_id, &text).await
        }
        ApiRequest::Play { device_id, url } => {
            if let Some(url) = url {
                xiaoai.play_url(&device_id, &url).await
            } else {
                xiaoai.set_play_state(&device_id, PlayState::Play).await
            }
        }
        ApiRequest::Pause { device_id } => {
            xiaoai.set_play_state(&device_id, PlayState::Pause).await
        }
        ApiRequest::Stop { device_id } => {
            xiaoai.set_play_state(&device_id, PlayState::Stop).await
        }
        ApiRequest::Volume { device_id, volume } => {
            xiaoai.set_volume(&device_id, volume).await
        }
        ApiRequest::Ask { device_id, text } => {
            xiaoai.nlp(&device_id, &text).await
        }
        ApiRequest::Status { device_id } => {
            match xiaoai.player_status_parsed(&device_id).await {
                Ok(status) => {
                    return ApiResponse::Success {
                        code: 0,
                        message: "OK".to_string(),
                        data: status.raw,
                    };
                }
                Err(e) => {
                    return ApiResponse::Error {
                        error: format!("获取状态失败: {}", e),
                    };
                }
            }
        }
        ApiRequest::GetDevices => {
            match xiaoai.device_info().await {
                Ok(devices) => {
                    let device_data = devices
                        .into_iter()
                        .map(|d| DeviceData {
                            device_id: d.device_id,
                            name: d.name,
                            hardware: d.hardware,
                        })
                        .collect();
                    
                    return ApiResponse::Devices {
                        devices: device_data,
                    };
                }
                Err(e) => {
                    return ApiResponse::Error {
                        error: format!("获取设备列表失败: {}", e),
                    };
                }
            }
        }
    };
    
    match result {
        Ok(response) => ApiResponse::Success {
            code: response.code,
            message: response.message,
            data: response.data,
        },
        Err(e) => ApiResponse::Error {
            error: format!("{}", e),
        },
    }
}
