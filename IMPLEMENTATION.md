# xiaoai-cli WebSocket API 实现总结

## 已实现的功能

### 1. API 模式配置
- ✅ 在 `config.json` 中添加 `api` 开关
- ✅ 在 `config.json` 中添加 `ws_port` 配置端口
- ✅ 在 `config.json` 中添加 `check` 开关控制关键词监听
- ✅ 在 `config.json` 中添加 `device_id` 和 `hardware` 配置监听设备

### 2. WebSocket 服务器
- ✅ 实现基于 tokio-tungstenite 的 WebSocket 服务器
- ✅ 支持多客户端并发连接
- ✅ 实现客户端管理和广播机制
- ✅ 支持异步消息处理

### 3. API 命令支持
实现了以下 8 个 WebSocket API 命令：

1. **get_devices** - 获取设备列表
2. **say** - 播报文本 (TTS)
3. **play** - 播放音乐/URL 或继续播放
4. **pause** - 暂停播放
5. **stop** - 停止播放
6. **volume** - 调整音量
7. **ask** - 向小爱发送自然语言问题
8. **status** - 获取播放器状态

### 4. 关键词监听与推送
- ✅ 支持在 API 模式下启动时自动监听关键词
- ✅ 当检测到关键词时，向所有连接的客户端广播消息
- ✅ 使用 `tokio::select!` 同时运行服务器和监听器
- ✅ 支持动态添加/移除客户端连接

### 5. 文档与示例
- ✅ 完整的 API 使用文档 (`API.md`)
- ✅ Python 客户端示例代码
- ✅ JavaScript 客户端示例代码
- ✅ 关键词监听的完整示例
- ✅ 测试脚本 (`test_api.py`)

## 配置示例

### 启用 API 模式（不启用监听）

```json
{
  "api": true,
  "ws_port": 8080,
  "check": false
}
```

### 启用 API 模式和关键词监听

```json
{
  "api": true,
  "ws_port": 8080,
  "check": true,
  "device_id": "123456789",
  "hardware": "L06A",
  "keywords": [
    "请问",
    "请帮我",
    "你好"
  ]
}
```

## 使用流程

1. **登录认证**
   ```bash
   cargo run -- login
   ```

2. **启动 API 服务器**
   ```bash
   cargo run
   ```
   服务器会：
   - 监听 WebSocket 连接（默认 8080 端口）
   - 如果启用了 `check`，自动开始关键词监听

3. **客户端连接**
   - 使用任何 WebSocket 客户端连接 `ws://localhost:8080`
   - 发送 JSON 格式的命令
   - 接收响应和关键词推送

## 消息类型

### 请求格式
```json
{
  "command": "命令名",
  "参数1": "值1",
  "参数2": "值2"
}
```

### 响应类型

1. **成功响应**
   ```json
   {
     "type": "success",
     "code": 0,
     "message": "OK",
     "data": {}
   }
   ```

2. **错误响应**
   ```json
   {
     "type": "error",
     "error": "错误信息"
   }
   ```

3. **设备列表**
   ```json
   {
     "type": "devices",
     "devices": [...]
   }
   ```

4. **关键词匹配（服务器主动推送）**
   ```json
   {
     "type": "keyword_match",
     "timestamp": 1635724800,
     "query": "用户说的话",
     "matched_keyword": "匹配到的关键词",
     "device_id": "设备ID"
   }
   ```

## 技术实现细节

### 并发模型
- 使用 Tokio 异步运行时
- 主服务器接受连接在独立任务中处理
- 关键词监听在独立任务中运行
- 使用 `Arc<RwLock<Vec>>` 管理客户端连接

### 广播机制
- 所有客户端连接存储在共享的 Vector 中
- 关键词匹配时遍历所有客户端发送消息
- 自动清理断开的客户端连接

### 错误处理
- 网络错误会被捕获并记录
- 客户端断开不影响服务器运行
- 监听失败会输出错误但不终止服务器

## 与 CLI 模式的关系

- API 模式和 CLI 模式互不冲突
- 根据 `config.json` 中的 `api` 字段自动选择模式
- CLI 模式仍然保留所有原有功能
- 可以通过不同的配置文件同时运行两种模式

## 限制和注意事项

1. `check` 功能需要 `api` 同时启用
2. 启用 `check` 时必须配置 `device_id` 和 `hardware`
3. 关键词监听使用的是 miai 库的 `ConversationWatcher`
4. 需要先登录才能使用 API 模式
5. WebSocket 连接断开后需要重新连接

## 测试方法

### 使用提供的测试脚本
```bash
python test_api.py                    # 运行基本测试
python test_api.py --interactive      # 交互式模式
```

### 使用 websocat
```bash
websocat ws://localhost:8080
{"command":"get_devices"}
```

### 使用浏览器
```javascript
const ws = new WebSocket('ws://localhost:8080');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
ws.send('{"command":"get_devices"}');
```

## 代码结构

```
cli/src/
├── main.rs           # 主入口，检测 API 模式并启动服务器
└── ws_server.rs      # WebSocket 服务器实现
    ├── WsServer      # 服务器结构体
    ├── ApiRequest    # API 请求枚举
    ├── ApiResponse   # API 响应枚举
    ├── handle_connection  # 处理单个客户端连接
    ├── handle_request     # 处理 API 命令
    └── broadcast_message  # 广播消息到所有客户端
```

## 未来可能的改进

1. 添加身份验证/授权机制
2. 支持 TLS/SSL 加密连接
3. 添加速率限制和请求限流
4. 支持更多的 API 命令
5. 添加 WebSocket ping/pong 心跳机制
6. 支持命令的异步回调
7. 添加 HTTP REST API 作为补充
