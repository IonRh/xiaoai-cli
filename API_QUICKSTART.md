# WebSocket API 模式快速开始

## ✨ 新功能：智能设备识别

现在启动 API 模式时，如果配置中没有指定 `device_id` 和 `hardware`，程序会自动从已登录账号获取设备信息！

## 🚀 快速开始

### 1. 登录

```bash
cargo run -- login
```

### 2. 配置关键词监听（可选）

编辑 `config.json`：

```json
{
  "username": "",
  "password": "",
  "api": false,
  "ws_port": 8080,
  "check": true,
  "device_id": "",
  "hardware": "",
  "keywords": [
    "请问",
    "请帮我",
    "你好",
    "嗨",
    "播放",
    "来首"
  ],
  "initial_interval": 1.0,
  "min_interval": 0.2,
  "max_interval": 3.0,
  "fetch_limit": 1,
  "block_xiaoai_response": true
}
```

**配置说明**：
- `ws_port`: WebSocket 服务器端口（默认 8080）
- `check`: 设置为 `true` 启用关键词监听
- `device_id` 和 `hardware`: 可以留空，程序会自动识别
- `keywords`: 简单的关键词字符串数组

### 3. 启动 WebSocket API 服务器

```bash
cargo run -- wsapi
```

你会看到类似输出：

```
🌐 启动 WebSocket API 服务器...
📱 未配置设备信息，正在自动获取...
✅ 自动选择唯一设备: 小爱音箱 (L06A)
🚀 WebSocket 服务器已启动
监听地址: ws://0.0.0.0:8080
🎧 开始监听关键词...
设备 ID: 123456789
设备型号: L06A
📝 已启用的关键词:
  1. 请问
  2. 请帮我
  3. 你好
```

## 📱 设备选择逻辑

1. **单个设备**：自动选择该设备
2. **多个设备**：自动选择第一个设备，并显示所有可用设备列表
3. **手动指定**：在 `config.json` 中设置 `device_id` 和 `hardware` 来指定特定设备

## 🔔 关键词监听推送

当启用 `check` 功能后，每当检测到关键词时，服务器会向所有连接的客户端推送消息：

```json
{
  "type": "keyword_match",
  "timestamp": 1635724800,
  "query": "请帮我打开空调",
  "matched_keyword": "请帮我",
  "device_id": "123456789"
}
```

## 💻 客户端示例

### Python 监听关键词

```python
import asyncio
import websockets
import json

async def listen():
    async with websockets.connect("ws://localhost:8080") as ws:
        print("等待关键词推送...")
        while True:
            msg = json.loads(await ws.recv())
            if msg["type"] == "keyword_match":
                print(f"🔔 用户说: {msg['query']}")
                # 响应用户
                await ws.send(json.dumps({
                    "command": "say",
                    "device_id": msg["device_id"],
                    "text": "好的，我听到了"
                }))

asyncio.run(listen())
```

## 📚 完整文档

查看 [API.md](./API.md) 了解所有 API 命令和详细说明。

## 🎯 使用场景

- 🏠 **智能家居集成**：将小爱音箱接入自己的智能家居系统
- 🤖 **自定义语音助手**：监听关键词，触发自定义逻辑
- 📱 **远程控制**：通过网络控制小爱音箱
- 🔗 **多平台集成**：Python、JavaScript、Go 等任何支持 WebSocket 的语言
