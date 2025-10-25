#!/usr/bin/env python3
"""
xiaoai-cli WebSocket API 测试客户端示例
"""

import asyncio
import websockets
import json
import sys


async def test_api():
    """测试 xiaoai-cli WebSocket API"""
    uri = "ws://localhost:8080"
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ 已连接到 WebSocket 服务器")
            print(f"📡 服务器地址: {uri}\n")
            
            # 1. 获取设备列表
            print("1️⃣  获取设备列表...")
            request = {"command": "get_devices"}
            await websocket.send(json.dumps(request))
            response = json.loads(await websocket.recv())
            
            if response["type"] == "error":
                print(f"❌ 错误: {response['error']}")
                return
            
            if response["type"] == "devices":
                print(f"✅ 找到 {len(response['devices'])} 个设备:")
                for device in response["devices"]:
                    print(f"   - {device['name']} (ID: {device['device_id']}, 型号: {device['hardware']})")
                
                if not response["devices"]:
                    print("❌ 没有可用设备")
                    return
                
                device_id = response["devices"][0]["device_id"]
                print(f"\n📱 使用设备: {response['devices'][0]['name']}\n")
                
                # 2. 播报文本
                print("2️⃣  测试 TTS (文本播报)...")
                request = {
                    "command": "say",
                    "device_id": device_id,
                    "text": "你好，我是小爱音箱 API 测试"
                }
                await websocket.send(json.dumps(request))
                response = json.loads(await websocket.recv())
                print_response("TTS", response)
                
                await asyncio.sleep(3)  # 等待播报完成
                
                # 3. 获取状态
                print("\n3️⃣  获取播放器状态...")
                request = {
                    "command": "status",
                    "device_id": device_id
                }
                await websocket.send(json.dumps(request))
                response = json.loads(await websocket.recv())
                print_response("状态", response)
                
                # 4. 调整音量
                print("\n4️⃣  调整音量到 30...")
                request = {
                    "command": "volume",
                    "device_id": device_id,
                    "volume": 30
                }
                await websocket.send(json.dumps(request))
                response = json.loads(await websocket.recv())
                print_response("音量", response)
                
                # 5. 询问
                print("\n5️⃣  测试询问功能...")
                request = {
                    "command": "ask",
                    "device_id": device_id,
                    "text": "现在几点了"
                }
                await websocket.send(json.dumps(request))
                response = json.loads(await websocket.recv())
                print_response("询问", response)
                
                print("\n✅ 所有测试完成!")
                
    except websockets.exceptions.WebSocketException as e:
        print(f"❌ WebSocket 连接错误: {e}")
        print("\n💡 提示:")
        print("   1. 确保服务器已启动: cargo run")
        print("   2. 确保在 config.json 中设置了 'api': true")
        print("   3. 确保已经登录: cargo run -- login")
    except Exception as e:
        print(f"❌ 错误: {e}")
        import traceback
        traceback.print_exc()


def print_response(title, response):
    """打印格式化的响应"""
    if response["type"] == "error":
        print(f"   ❌ {title} 失败: {response['error']}")
    elif response["type"] == "success":
        print(f"   ✅ {title} 成功")
        print(f"   📊 Code: {response['code']}, Message: {response['message']}")
        if response.get("data"):
            data_str = json.dumps(response["data"], indent=6, ensure_ascii=False)
            print(f"   📦 Data: {data_str}")


async def interactive_mode():
    """交互式模式"""
    uri = "ws://localhost:8080"
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ 已连接到 WebSocket 服务器")
            print(f"📡 服务器地址: {uri}")
            print("💬 进入交互模式，输入 JSON 格式的命令，输入 'quit' 退出\n")
            
            # 先获取设备列表
            request = {"command": "get_devices"}
            await websocket.send(json.dumps(request))
            response = json.loads(await websocket.recv())
            
            if response["type"] == "devices" and response["devices"]:
                print("📱 可用设备:")
                for i, device in enumerate(response["devices"], 1):
                    print(f"   {i}. {device['name']} (ID: {device['device_id']})")
                print()
            
            while True:
                try:
                    command = input(">>> ")
                    if command.lower() in ['quit', 'exit', 'q']:
                        break
                    
                    if not command.strip():
                        continue
                    
                    # 解析 JSON
                    request = json.loads(command)
                    await websocket.send(json.dumps(request))
                    response = json.loads(await websocket.recv())
                    
                    print(json.dumps(response, indent=2, ensure_ascii=False))
                    print()
                    
                except json.JSONDecodeError as e:
                    print(f"❌ JSON 格式错误: {e}\n")
                except KeyboardInterrupt:
                    print("\n👋 再见!")
                    break
                    
    except websockets.exceptions.WebSocketException as e:
        print(f"❌ WebSocket 连接错误: {e}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--interactive":
        print("🎮 启动交互式模式\n")
        asyncio.run(interactive_mode())
    else:
        print("🧪 运行 API 测试\n")
        asyncio.run(test_api())
        print("\n💡 提示: 使用 --interactive 参数启动交互式模式")
