# xiaoai-cli WebSocket API 使用文档

## 概述

xiaoai-cli 支持通过 WebSocket 提供 API 服务，可以通过网络调用控制小爱音箱。

## 启用 API 模式

在 `config.json` 中设置以下配置：

```json
{
  "username": "",
  "password": "",
  "ws_port": 8080,
  "check": true,
  "device_id": "",
  "hardware": "",
  "keywords": [
    "请问",
    "请帮我",
    "你好"
  ],
  "initial_interval": 1.0,
  "min_interval": 0.2,
  "max_interval": 3.0,
  "fetch_limit": 1,
  "block_xiaoai_response": true
}
```

**配置说明：**
- `ws_port`: WebSocket 服务器监听端口（默认 8080）
- `check`: 设置为 `true` 启用关键词监听功能
- `device_id`: 监听的设备 ID（**可选**，留空时自动获取）
- `hardware`: 设备型号（**可选**，留空时自动获取，如 "L06A", "L05C" 等）
- `keywords`: 要监听的关键词列表（简单字符串数组）
- 其他配置项控制监听行为

**关键词配置支持两种格式：**

1. **简单格式**（推荐）：
```json
"keywords": ["请问", "请帮我", "你好"]
```

2. **高级格式**（支持更多选项）：
```json
"keywords": [
  {
    "keywords": ["请问", "请帮"],
    "match_mode": "starts_with",
    "enabled": true,
    "description": "礼貌询问"
  }
]
```

**设备自动选择：**
- 如果 `device_id` 和 `hardware` 留空，程序会自动从已登录账号获取设备列表
- 如果只有一个设备，会自动选择该设备
- 如果有多个设备，会自动选择第一个设备，并在控制台显示所有可用设备
- 建议：如果有多个设备，可以手动在配置中指定要监听的设备

## 启动服务器

首先需要登录：

```bash
cargo run -- login
```

然后启动 WebSocket API 服务器：

```bash
cargo run -- wsapi
```

服务器会在 `ws://0.0.0.0:8080` 上监听连接。

## 配置说明

## API 请求格式

所有请求都是 JSON 格式，通过 WebSocket 发送。请求格式：

```json
{
  "command": "命令名称",
  "参数1": "值1",
  "参数2": "值2"
}
```

## API 响应格式

### 成功响应

```json
{
  "type": "success",
  "code": 0,
  "message": "OK",
  "data": {}
}
```

### 错误响应

```json
{
  "type": "error",
  "error": "错误信息"
}
```

### 设备列表响应

```json
{
  "type": "devices",
  "devices": [
    {
      "device_id": "设备ID",
      "name": "设备名称",
      "hardware": "机型"
    }
  ]
}
```

### 关键词匹配推送（当启用 check 时）

当检测到关键词时，服务器会主动向所有连接的客户端推送此消息：

```json
{
  "type": "keyword_match",
  "timestamp": 1635724800,
  "query": "用户说的话",
  "matched_keyword": "匹配到的关键词",
  "device_id": "设备ID"
}
```

## 支持的命令

### 1. 获取设备列表

获取所有小爱音箱设备。

**请求：**
```json
{
  "command": "get_devices"
}
```

**响应示例：**
```json
{
  "type": "devices",
  "devices": [
    {
      "device_id": "123456789",
      "name": "小爱音箱",
      "hardware": "L06A"
    }
  ]
}
```

### 2. 播报文本 (TTS)

让小爱音箱播报指定文本。

**请求：**
```json
{
  "command": "say",
  "device_id": "123456789",
  "text": "你好，世界"
}
```

### 3. 播放音乐/URL

播放指定 URL 或继续播放。

**请求（播放 URL）：**
```json
{
  "command": "play",
  "device_id": "123456789",
  "url": "http://example.com/music.mp3"
}
```

**请求（继续播放）：**
```json
{
  "command": "play",
  "device_id": "123456789",
  "url": null
}
```

### 4. 暂停播放

暂停当前播放的内容。

**请求：**
```json
{
  "command": "pause",
  "device_id": "123456789"
}
```

### 5. 停止播放

停止当前播放的内容。

**请求：**
```json
{
  "command": "stop",
  "device_id": "123456789"
}
```

### 6. 调整音量

设置音箱音量（0-100）。

**请求：**
```json
{
  "command": "volume",
  "device_id": "123456789",
  "volume": 50
}
```

### 7. 询问小爱

向小爱发送自然语言问题。

**请求：**
```json
{
  "command": "ask",
  "device_id": "123456789",
  "text": "今天天气怎么样"
}
```

### 8. 获取播放状态

获取当前播放器状态信息。

**请求：**
```json
{
  "command": "status",
  "device_id": "123456789"
}
```

**响应示例：**
```json
{
  "type": "success",
  "code": 0,
  "message": "OK",
  "data": {
    "status": "playing",
    "volume": 50,
    "current_track": "音乐名称"
  }
}
```

## Python 客户端示例

### 基本使用示例

```python
import asyncio
import websockets
import json

async def control_xiaoai():
    uri = "ws://localhost:8080"
    
    async with websockets.connect(uri) as websocket:
        # 获取设备列表
        request = {"command": "get_devices"}
        await websocket.send(json.dumps(request))
        response = json.loads(await websocket.recv())
        print("设备列表:", response)
        
        if response["type"] == "devices" and response["devices"]:
            device_id = response["devices"][0]["device_id"]
            
            # 播报文本
            request = {
                "command": "say",
                "device_id": device_id,
                "text": "你好，我是小爱"
            }
            await websocket.send(json.dumps(request))
            response = json.loads(await websocket.recv())
            print("播报响应:", response)
            
            # 获取状态
            request = {
                "command": "status",
                "device_id": device_id
            }
            await websocket.send(json.dumps(request))
            response = json.loads(await websocket.recv())
            print("状态:", response)

# 运行示例
asyncio.run(control_xiaoai())
```

### 监听关键词推送示例

当启用 `check` 功能后，服务器会主动推送关键词匹配消息：

```python
import asyncio
import websockets
import json

async def listen_keywords():
    uri = "ws://localhost:8080"
    
    async with websockets.connect(uri) as websocket:
        print("已连接到服务器，等待关键词推送...")
        
        while True:
            try:
                message = await websocket.recv()
                data = json.loads(message)
                
                if data["type"] == "keyword_match":
                    print(f"\n🔔 检测到关键词!")
                    print(f"  时间: {data['timestamp']}")
                    print(f"  用户说: {data['query']}")
                    print(f"  匹配关键词: {data['matched_keyword']}")
                    print(f"  设备ID: {data['device_id']}")
                    
                    # 在这里可以触发自定义的回调逻辑
                    # 例如：调用 say 命令回复用户
                    response_request = {
                        "command": "say",
                        "device_id": data["device_id"],
                        "text": f"我听到你说了：{data['matched_keyword']}"
                    }
                    await websocket.send(json.dumps(response_request))
                    
                elif data["type"] == "success":
                    print(f"✅ 命令执行成功: {data['message']}")
                elif data["type"] == "error":
                    print(f"❌ 错误: {data['error']}")
                    
            except websockets.exceptions.ConnectionClosed:
                print("连接已关闭")
                break
            except Exception as e:
                print(f"错误: {e}")

# 运行监听
asyncio.run(listen_keywords())
```

## JavaScript 客户端示例

```javascript
const ws = new WebSocket('ws://localhost:8080');

ws.onopen = async () => {
    console.log('已连接到服务器');
    
    // 获取设备列表
    ws.send(JSON.stringify({
        command: 'get_devices'
    }));
};

ws.onmessage = async (event) => {
    const response = JSON.parse(event.data);
    console.log('收到响应:', response);
    
    if (response.type === 'devices' && response.devices.length > 0) {
        const deviceId = response.devices[0].device_id;
        
        // 播报文本
        ws.send(JSON.stringify({
            command: 'say',
            device_id: deviceId,
            text: '你好，世界'
        }));
    }
};

ws.onerror = (error) => {
    console.error('WebSocket 错误:', error);
};

ws.onclose = () => {
    console.log('连接已关闭');
};
```

## 注意事项

1. **认证要求**：在启动 API 模式前，必须先运行 `cargo run -- login` 完成登录
2. **设备 ID**：大部分命令都需要指定 `device_id`，可通过 `get_devices` 命令获取
3. **连接保持**：WebSocket 连接会保持打开状态，可以连续发送多个请求
4. **错误处理**：建议在客户端实现重连机制和错误处理
5. **关键词监听**：
   - 启用 `check` 功能后，服务器会自动监听配置的设备
   - 当检测到关键词时，会向所有连接的客户端广播 `keyword_match` 消息
   - `device_id` 和 `hardware` 可以留空，程序会自动获取设备信息
   - 如果有多个设备且希望指定特定设备，可在配置中手动设置 `device_id` 和 `hardware`
   - 监听功能与服务器同时运行，无需额外命令

## 测试工具

可以使用以下工具测试 WebSocket API：

- **websocat**: 命令行 WebSocket 客户端
  ```bash
  websocat ws://localhost:8080
  ```
  
- **Postman**: 支持 WebSocket 的 API 测试工具

- **浏览器开发者工具**: 在浏览器控制台直接测试

## 故障排除

1. **连接失败**：确保服务器已启动且端口未被占用
2. **认证错误**：检查 `xiaoai-auth.json` 文件是否存在且有效
3. **设备不响应**：确认设备 ID 正确，设备在线且已绑定到账号
