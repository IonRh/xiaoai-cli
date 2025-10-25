"""
Flask 示例：通过 HTTP REST API 控制小爱音箱

这个示例展示了如何创建一个 Flask Web 服务，
将 HTTP 请求转发到 xiaoai-cli 的 WebSocket API。

使用方法：
1. 启动 xiaoai-cli WebSocket 服务器:
   cargo run -- wsapi

2. 启动 Flask 服务器:
   python flask_example.py

3. 访问 http://localhost:5000 查看 Web 界面
   或使用 API 端点进行控制

依赖安装：
pip install flask websockets
"""

from flask import Flask, request, jsonify, render_template_string
import asyncio
import websockets
import json
from threading import Thread, Lock
import queue
import time

app = Flask(__name__)

# WebSocket 服务器地址
WS_URL = "ws://localhost:8080"

# 全局 WebSocket 连接
ws_connection = None
ws_lock = Lock()
response_queue = queue.Queue()
keyword_queue = queue.Queue()

# 请求ID计数器
request_id_counter = 0
pending_requests = {}


class WebSocketManager:
    """WebSocket 连接管理器"""
    
    def __init__(self, url):
        self.url = url
        self.websocket = None
        self.connected = False
        self.loop = None
        self.receive_task = None
        
    async def connect(self):
        """建立并维护 WebSocket 连接"""
        while True:
            try:
                print(f"🔌 正在连接到 WebSocket 服务器: {self.url}")
                async with websockets.connect(self.url) as websocket:
                    self.websocket = websocket
                    self.connected = True
                    print("✅ WebSocket 连接已建立")
                    
                    # 启动消息接收任务
                    self.receive_task = asyncio.create_task(self.receive_messages())
                    
                    # 等待连接断开
                    await self.receive_task
                        
            except websockets.exceptions.ConnectionClosed:
                print("⚠️  WebSocket 连接已关闭，5秒后重连...")
                self.connected = False
                await asyncio.sleep(5)
            except Exception as e:
                print(f"❌ WebSocket 连接错误: {e}")
                self.connected = False
                await asyncio.sleep(5)
    
    async def receive_messages(self):
        """独立的消息接收协程"""
        try:
            async for message in self.websocket:
                await self.handle_message(message)
        except Exception as e:
            print(f"❌ 接收消息错误: {e}")
            self.connected = False
    
    async def handle_message(self, message):
        """处理接收到的消息"""
        try:
            print(f"📩 收到原始消息: {message}")
            data = json.loads(message)
            print(f"📦 解析后的数据: {json.dumps(data, indent=2)}")
            
            # 如果是关键词匹配推送
            if data.get("type") == "keyword_match":
                print(f"\n🔔 关键词匹配: {data.get('matched_keyword')} - {data.get('query')}")
                keyword_queue.put(data)
            else:
                # 普通响应消息
                print(f"✅ 放入响应队列")
                response_queue.put(data)
                
        except Exception as e:
            print(f"❌ 消息处理错误: {e}")
            import traceback
            traceback.print_exc()
    
    async def send_command(self, command_data, timeout=10):
        """发送命令并等待响应"""
        if not self.connected or not self.websocket:
            return {"error": "WebSocket 未连接"}
        
        try:
            # 清空旧的响应（如果有）
            while not response_queue.empty():
                try:
                    response_queue.get_nowait()
                except queue.Empty:
                    break
            
            # 发送命令
            await self.websocket.send(json.dumps(command_data))
            print(f"📨 发送命令: {json.dumps(command_data)}")
            
            # 等待响应 - 使用更短的检查间隔
            start_time = time.time()
            while time.time() - start_time < timeout:
                try:
                    response = response_queue.get(timeout=0.1)
                    print(f"📥 收到响应: {json.dumps(response)}")
                    return response
                except queue.Empty:
                    # 让出控制权，允许其他协程运行
                    await asyncio.sleep(0.01)
                    continue
            
            print(f"⏱️  等待响应超时 ({timeout}秒)")
            return {"error": "等待响应超时"}
            
        except Exception as e:
            print(f"❌ 发送命令错误: {e}")
            import traceback
            traceback.print_exc()
            return {"error": str(e)}
    
    def run(self):
        """在独立线程中运行事件循环"""
        self.loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self.loop)
        self.loop.run_until_complete(self.connect())


# 创建全局 WebSocket 管理器
ws_manager = WebSocketManager(WS_URL)


def send_ws_command(command_data):
    """同步发送命令到 WebSocket（使用全局连接）"""
    if not ws_manager.connected:
        return {"error": "WebSocket 未连接，请稍后重试"}
    
    # 在 WebSocket 的事件循环中执行
    future = asyncio.run_coroutine_threadsafe(
        ws_manager.send_command(command_data),
        ws_manager.loop
    )
    
    try:
        return future.result(timeout=10)
    except Exception as e:
        return {"error": str(e)}


# ============ REST API 端点 ============

@app.route('/')
def index():
    """Web 控制界面"""
    html = """
    <!DOCTYPE html>
    <html>
    <head>
        <title>小爱音箱控制面板</title>
        <meta charset="utf-8">
        <style>
            body {
                font-family: Arial, sans-serif;
                max-width: 800px;
                margin: 50px auto;
                padding: 20px;
                background: #f5f5f5;
            }
            .container {
                background: white;
                padding: 30px;
                border-radius: 10px;
                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            }
            h1 {
                color: #333;
                text-align: center;
            }
            .section {
                margin: 20px 0;
                padding: 20px;
                background: #f9f9f9;
                border-radius: 5px;
            }
            input, select {
                width: 100%;
                padding: 10px;
                margin: 5px 0;
                border: 1px solid #ddd;
                border-radius: 4px;
                box-sizing: border-box;
            }
            button {
                background: #4CAF50;
                color: white;
                padding: 12px 24px;
                border: none;
                border-radius: 4px;
                cursor: pointer;
                margin: 5px;
                font-size: 14px;
            }
            button:hover {
                background: #45a049;
            }
            .danger {
                background: #f44336;
            }
            .danger:hover {
                background: #da190b;
            }
            #result {
                margin-top: 20px;
                padding: 15px;
                background: #e8f5e9;
                border-radius: 4px;
                white-space: pre-wrap;
                font-family: monospace;
                font-size: 12px;
            }
            .device-info {
                background: #e3f2fd;
                padding: 10px;
                border-radius: 4px;
                margin: 10px 0;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>🎙️ 小爱音箱控制面板</h1>
            
            <div class="section">
                <h3>1. 获取设备列表</h3>
                <button onclick="getDevices()">📱 获取设备</button>
                <div id="devices"></div>
            </div>
            
            <div class="section">
                <h3>2. 文本播报 (TTS)</h3>
                <input type="text" id="tts-text" placeholder="输入要播报的文本" value="你好，这是测试消息">
                <button onclick="say()">🔊 播报</button>
            </div>
            
            <div class="section">
                <h3>3. 播放控制</h3>
                <input type="text" id="play-url" placeholder="音乐 URL (可选)" value="">
                <button onclick="play()">▶️ 播放</button>
                <button onclick="pause()">⏸️ 暂停</button>
                <button onclick="stop()" class="danger">⏹️ 停止</button>
            </div>
            
            <div class="section">
                <h3>4. 音量控制</h3>
                <input type="range" id="volume" min="0" max="100" value="50" oninput="document.getElementById('volume-value').innerText=this.value">
                <span>音量: <span id="volume-value">50</span></span>
                <button onclick="setVolume()">🔊 设置音量</button>
            </div>
            
            <div class="section">
                <h3>5. 询问小爱</h3>
                <input type="text" id="ask-text" placeholder="输入要询问的问题" value="今天天气怎么样">
                <button onclick="ask()">❓ 询问</button>
            </div>
            
            <div class="section">
                <h3>6. 获取状态</h3>
                <button onclick="getStatus()">📊 获取播放状态</button>
            </div>
            
            <div id="result"></div>
        </div>
        
        <script>
            let currentDeviceId = null;
            
            function showResult(data) {
                document.getElementById('result').innerText = JSON.stringify(data, null, 2);
            }
            
            async function apiCall(endpoint, data = {}) {
                try {
                    const response = await fetch(endpoint, {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify(data)
                    });
                    const result = await response.json();
                    showResult(result);
                    return result;
                } catch (error) {
                    showResult({error: error.message});
                }
            }
            
            async function getDevices() {
                const result = await apiCall('/api/devices');
                if (result.devices && result.devices.length > 0) {
                    currentDeviceId = result.devices[0].device_id;
                    const devicesDiv = document.getElementById('devices');
                    devicesDiv.innerHTML = '<div class="device-info">' +
                        result.devices.map((d, i) => 
                            `<div><strong>${i + 1}. ${d.name}</strong><br>` +
                            `ID: ${d.device_id}<br>型号: ${d.hardware}</div>`
                        ).join('<hr>') + '</div>';
                }
            }
            
            async function say() {
                const text = document.getElementById('tts-text').value;
                if (!currentDeviceId) await getDevices();
                await apiCall('/api/say', {device_id: currentDeviceId, text: text});
            }
            
            async function play() {
                if (!currentDeviceId) await getDevices();
                const url = document.getElementById('play-url').value || null;
                await apiCall('/api/play', {device_id: currentDeviceId, url: url});
            }
            
            async function pause() {
                if (!currentDeviceId) await getDevices();
                await apiCall('/api/pause', {device_id: currentDeviceId});
            }
            
            async function stop() {
                if (!currentDeviceId) await getDevices();
                await apiCall('/api/stop', {device_id: currentDeviceId});
            }
            
            async function setVolume() {
                if (!currentDeviceId) await getDevices();
                const volume = parseInt(document.getElementById('volume').value);
                await apiCall('/api/volume', {device_id: currentDeviceId, volume: volume});
            }
            
            async function ask() {
                const text = document.getElementById('ask-text').value;
                if (!currentDeviceId) await getDevices();
                await apiCall('/api/ask', {device_id: currentDeviceId, text: text});
            }
            
            async function getStatus() {
                if (!currentDeviceId) await getDevices();
                await apiCall('/api/status', {device_id: currentDeviceId});
            }
            
            // 页面加载时自动获取设备
            window.onload = getDevices;
        </script>
    </body>
    </html>
    """
    return render_template_string(html)


@app.route('/api/devices', methods=['GET', 'POST'])
def get_devices():
    """获取设备列表"""
    command = {"command": "get_devices"}
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/say', methods=['POST'])
def say():
    """播报文本"""
    data = request.json
    device_id = data.get('device_id')
    text = data.get('text', '你好')
    
    command = {
        "command": "say",
        "device_id": device_id,
        "text": text
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/play', methods=['POST'])
def play():
    """播放"""
    data = request.json
    device_id = data.get('device_id')
    url = data.get('url')
    
    command = {
        "command": "play",
        "device_id": device_id,
        "url": url
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/pause', methods=['POST'])
def pause():
    """暂停"""
    data = request.json
    device_id = data.get('device_id')
    
    command = {
        "command": "pause",
        "device_id": device_id
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/stop', methods=['POST'])
def stop():
    """停止"""
    data = request.json
    device_id = data.get('device_id')
    
    command = {
        "command": "stop",
        "device_id": device_id
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/volume', methods=['POST'])
def set_volume():
    """设置音量"""
    data = request.json
    device_id = data.get('device_id')
    volume = data.get('volume', 50)
    
    command = {
        "command": "volume",
        "device_id": device_id,
        "volume": volume
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/ask', methods=['POST'])
def ask():
    """询问小爱"""
    data = request.json
    device_id = data.get('device_id')
    text = data.get('text', '今天天气怎么样')
    
    command = {
        "command": "ask",
        "device_id": device_id,
        "text": text
    }
    result = send_ws_command(command)
    return jsonify(result)


@app.route('/api/status', methods=['POST'])
def status():
    """获取播放状态"""
    data = request.json
    device_id = data.get('device_id')
    
    command = {
        "command": "status",
        "device_id": device_id
    }
    result = send_ws_command(command)
    return jsonify(result)


# ============ 关键词监听 WebSocket 客户端 ============

async def listen_keywords():
    """持续监听 WebSocket 推送的关键词匹配消息"""
    while True:
        try:
            async with websockets.connect(WS_URL) as websocket:
                print("🎧 开始监听关键词推送...")
                while True:
                    message = await websocket.recv()
                    data = json.loads(message)
                    
                    if data.get("type") == "keyword_match":
                        print(f"\n🔔 关键词匹配!")
                        print(f"  用户说: {data.get('query')}")
                        print(f"  匹配关键词: {data.get('matched_keyword')}")
                        
                        # 将消息放入队列
                        message_queue.put(data)
                        
        except Exception as e:
            print(f"WebSocket 监听错误: {e}")
            await asyncio.sleep(5)  # 等待 5 秒后重连


def start_ws_listener():
    """在后台线程中启动 WebSocket 监听"""
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    loop.run_until_complete(listen_keywords())


@app.route('/api/keywords/latest', methods=['GET'])
def get_latest_keyword():
    """获取最新的关键词匹配消息"""
    try:
        # 非阻塞获取
        message = message_queue.get_nowait()
        return jsonify(message)
    except queue.Empty:
        return jsonify({"message": "没有新消息"}), 204


if __name__ == '__main__':
    print("=" * 50)
    print("🚀 Flask 控制服务器启动中...")
    print("=" * 50)
    print("\n📝 使用步骤:")
    print("1. 确保 xiaoai-cli WebSocket 服务器正在运行:")
    print("   cd /workspaces/xiaoai-cli")
    print("   cargo run -- wsapi")
    print("\n2. 访问 Web 界面:")
    print("   http://localhost:5000")
    print("\n3. 或使用 API 端点，例如:")
    print("   curl -X POST http://localhost:5000/api/say \\")
    print("        -H 'Content-Type: application/json' \\")
    print("        -d '{\"device_id\":\"YOUR_DEVICE_ID\",\"text\":\"你好\"}'")
    print("\n" + "=" * 50)
    
    # 启动 WebSocket 管理器线程
    ws_thread = Thread(target=ws_manager.run, daemon=True)
    ws_thread.start()
    print("\n🔌 WebSocket 管理器线程已启动，正在连接...")
    
    # 等待一下让连接建立
    time.sleep(2)
    
    # 启动 Flask 服务器
    app.run(debug=True, host='0.0.0.0', port=5000, use_reloader=False)
