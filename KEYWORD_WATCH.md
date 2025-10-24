# 小爱音箱关键词监听功能

本功能实现了类似 mi-gpt 的动态间隔轮询和关键词检测机制，可以监听小爱音箱的对话并在检测到指定关键词时触发回调。

## 特性

- ✅ **动态间隔轮询**：有新消息时加快检测（0.5秒），无消息时逐渐降低频率（最高3秒）
- ✅ **智能去重**：使用时间戳跟踪，避免重复处理相同对话
- ✅ **灵活匹配**：支持前缀匹配（`starts_with`）、包含匹配（`contains`）、精确匹配（`exact`）
- ✅ **自动阻断**：检测到关键词后自动暂停小爱的默认回复
- ✅ **配置化管理**：使用 JSON 配置文件管理关键词

## 快速开始

### 1. 创建配置文件

复制示例配置：

```bash
cp keywords.example.json keywords.json
```

编辑 `keywords.json`：

```json
{
  "keywords": [
    {
      "keywords": ["请问", "请帮我"],
      "match_mode": "starts_with",
      "enabled": true,
      "description": "礼貌询问"
    },
    {
      "keywords": ["播放", "来首"],
      "match_mode": "starts_with",
      "enabled": true,
      "description": "音乐播放"
    }
  ],
  "initial_interval": 1.0,
  "min_interval": 0.5,
  "max_interval": 3.0,
  "fetch_limit": 5,
  "block_xiaoai_response": true
}
```

### 2. 启动监听

```bash
# 使用默认配置文件 keywords.json
xiaoai watch

# 使用自定义配置文件
xiaoai watch --config my-keywords.json

# 指定设备
xiaoai watch --device-id YOUR_DEVICE_ID
```

### 3. 输出格式

当检测到关键词时，程序会输出 JSON 格式的匹配信息：

```json
{
  "timestamp": 1729785600,
  "query": "请问今天天气怎么样",
  "matched_keyword": "请问",
  "description": "礼貌询问",
  "device_id": "123456789"
}
```

## 配置说明

### 关键词配置 (`KeywordConfig`)

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `keywords` | `string[]` | 关键词列表 | **必填** |
| `match_mode` | `enum` | 匹配模式：`starts_with`/`contains`/`exact` | `starts_with` |
| `enabled` | `boolean` | 是否启用 | `true` |
| `description` | `string` | 关键词描述 | `""` |

### 监听器配置 (`WatcherConfig`)

| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `keywords` | `KeywordConfig[]` | 关键词配置列表 | `[]` |
| `initial_interval` | `number` | 初始轮询间隔（秒） | `1.0` |
| `min_interval` | `number` | 最小轮询间隔（秒） | `0.5` |
| `max_interval` | `number` | 最大轮询间隔（秒） | `3.0` |
| `fetch_limit` | `number` | 单次拉取对话数量 | `5` |
| `block_xiaoai_response` | `boolean` | 是否阻断小爱默认回复 | `true` |

## 匹配模式说明

### `starts_with` - 前缀匹配（推荐）
精确度高，适合命令式关键词。

```json
{
  "keywords": ["播放", "打开"],
  "match_mode": "starts_with"
}
```

匹配：
- ✅ "播放一首歌"
- ✅ "打开空调"
- ❌ "我想播放"（不是开头）

### `contains` - 包含匹配
灵活但可能误触。

```json
{
  "keywords": ["音乐", "歌曲"],
  "match_mode": "contains"
}
```

匹配：
- ✅ "播放音乐"
- ✅ "我想听歌曲"
- ✅ "这首歌曲很好听"

### `exact` - 精确匹配
最严格，适合特定命令。

```json
{
  "keywords": ["停止", "暂停"],
  "match_mode": "exact"
}
```

匹配：
- ✅ "停止"
- ✅ "暂停"
- ❌ "停止播放"（不完全一样）

## 高级用法

### 与其他脚本集成

监听输出为 JSON 格式，可以轻松与其他脚本集成：

```bash
# 使用 jq 处理输出
xiaoai watch | jq -r '.query'

# Python 集成
xiaoai watch | python process_keywords.py

# Node.js 集成
xiaoai watch | node process_keywords.js
```

### Python 示例

```python
#!/usr/bin/env python3
import sys
import json

for line in sys.stdin:
    try:
        data = json.loads(line)
        query = data['query']
        keyword = data['matched_keyword']
        
        print(f"检测到: {keyword} -> {query}")
        
        # 根据关键词执行不同操作
        if keyword.startswith("播放"):
            # 处理播放请求
            pass
        elif keyword.startswith("请问"):
            # 处理问答请求
            pass
            
    except json.JSONDecodeError:
        continue
```

### Node.js 示例

```javascript
#!/usr/bin/env node
const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', (line) => {
  try {
    const data = JSON.parse(line);
    console.log(`检测到: ${data.matched_keyword} -> ${data.query}`);
    
    // 根据关键词执行不同操作
    if (data.matched_keyword.startsWith('播放')) {
      // 处理播放请求
    }
  } catch (e) {
    // 忽略非 JSON 行
  }
});
```

## 实现原理

### 动态间隔调整

参考 mi-gpt 的实现，监听器会根据消息活跃度自动调整轮询间隔：

```
有新消息 → 间隔 = 0.5s（快速响应）
无新消息 → 间隔 *= 1.2（逐渐降低）
最大间隔 → 3.0s（节省资源）
```

### 消息去重

使用 `HashSet<i64>` 存储已处理消息的时间戳，避免重复处理：

```rust
if !self.seen_timestamps.contains(&conv.time) {
    self.seen_timestamps.insert(conv.time);
    // 处理新消息
}
```

### 阻断机制

检测到关键词后，立即调用暂停播放接口阻止小爱默认回复：

```rust
if self.config.block_xiaoai_response {
    xiaoai.set_play_state(device_id, PlayState::Pause).await?;
}
```

## 故障排查

### 监听不到消息

1. 检查设备 ID 和型号是否正确
2. 确认认证文件有效（`xiaoai device` 查看设备列表）
3. 尝试手动触发对话，观察日志输出

### 关键词不匹配

1. 检查匹配模式是否正确（`starts_with` vs `contains`）
2. 注意大小写和标点符号
3. 启用 `RUST_LOG=debug` 查看详细日志

### 阻断失败

某些型号可能不支持播放控制，可以禁用阻断功能：

```json
{
  "block_xiaoai_response": false
}
```

## 相关文档

- [CLI 使用文档](../README.md)
- [API 文档](../miai/README.md)
- [mi-gpt 项目](https://github.com/idootop/mi-gpt)

## 许可证

MIT License
