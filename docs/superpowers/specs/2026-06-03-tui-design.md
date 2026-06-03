# TranRSS TUI + CLI 统一架构设计

## 概述

将 TranRSS 改造为统一的 CLI/TUI 工具，替代原有的纯 Web 模式。核心改动：

1. **统一入口** `tranrss` — 管理所有服务（Web、定时任务、TUI）
2. **系统级定时任务** — 检测 systemd/Alpine，使用 OS 原生调度替代 Apalis
3. **统一日志** — systemd 用 journald，Alpine 用 syslog
4. **TUI 交互界面** — Ratatui 标签页式，复刻网页核心功能
5. **CLI 子命令** — 子命令风格，管理类操作用命令行

## 架构

```
tranrss (统一二进制)
├── serve          # 启动所有服务（Web + 定时 + 可选 TUI）
├── tui            # 纯 TUI 交互界面
├── sub            # 订阅管理 CLI
├── article        # 文章操作 CLI
├── job            # 任务队列 CLI
├── api            # API 配置 CLI
├── config         # 配置管理 CLI
└── setup          # 初始化/安装（生成 systemd unit / crontab）
```

## 详细设计

### 1. `tranrss serve` — 统一服务启动

```bash
tranrss serve                    # 启动 Web + 定时任务
tranrss serve --with-tui         # 启动 Web + 定时任务 + TUI
tranrss serve --port 8000        # 指定端口
tranrss serve --no-cron          # 不启动定时任务
```

功能：
- 启动 Axum Web 服务（现有逻辑）
- 启动系统定时任务调度（替代 Apalis cron）
- 可选启动 TUI（后台线程，前台接管终端）

### 2. 系统级定时任务

#### 检测逻辑

```rust
fn detect_scheduler() -> Scheduler {
    if Path::new("/run/systemd/system").exists() {
        Scheduler::Systemd
    } else if Path::new("/etc/init.d").exists() {
        Scheduler::AlpineCron
    } else {
        Scheduler::Fallback  // 内置 tokio 定时
    }
}
```

#### `tranrss setup` — 生成系统配置

```bash
tranrss setup                    # 自动检测并生成
tranrss setup --scheduler systemd
tranrss setup --scheduler alpine
```

**systemd 输出：**
```
/etc/systemd/system/tranrss.service    # 主服务 unit
/etc/systemd/system/tranrss-sync.timer # 订阅同步定时器
/etc/systemd/system/tranrss-sync.service
```

```ini
# tranrss-sync.timer
[Unit]
Description=TranRSS Feed Sync Timer

[Timer]
OnBootSec=1min
OnUnitActiveSec=30min
Persistent=true

[Install]
WantedBy=timers.target
```

```ini
# tranrss-sync.service
[Unit]
Description=TranRSS Feed Sync

[Service]
Type=oneshot
ExecStart=/usr/local/bin/tranrss cron sync
User=tranrss
```

**Alpine 输出：**
```
/etc/init.d/tranrss              # OpenRC init script
/etc/periodic/15min/tranrss-sync # crontab 条目
```

```sh
#!/bin/sh
# /etc/periodic/15min/tranrss-sync
/usr/local/bin/tranrss cron sync
```

#### `tranrss cron` — 定时任务执行入口

```bash
tranrss cron sync                # 执行一次订阅同步
tranrss cron translate           # 执行待翻译文章
tranrss cron summarize           # 执行待摘要文章
```

系统调度器调用这些命令，后端只负责执行，不负责调度。

### 3. 日志系统

#### 配置

```toml
# ~/.config/tranrss/config.toml
[log]
output = "auto"     # auto / journald / syslog / file / stdout
level = "info"      # trace / debug / info / warn / error
file = "/var/log/tranrss/tranrss.log"  # output=file 时使用
```

#### 自动检测

```rust
fn detect_log_output() -> LogOutput {
    if std::env::var("JOURNAL_STREAM").is_ok() {
        LogOutput::Journald  // 由 systemd 启动
    } else if Path::new("/dev/log").exists() {
        LogOutput::Syslog
    } else {
        LogOutput::Stdout
    }
}
```

#### 实现

- systemd: `tracing-journald` crate
- syslog: `tracing-syslog` crate
- file: `tracing-appender` crate
- stdout: 现有 `tracing-subscriber`

### 4. CLI 子命令

```bash
# 订阅管理
tranrss sub list
tranrss sub add <url> [--category X] [--translate] [--summary]
tranrss sub edit <id> [--title X] [--category X]
tranrss sub delete <id>
tranrss sub sync [id]            # 不传 id 同步全部
tranrss sub inactive

# 文章操作
tranrss article list [--feed <id|name>] [--unread] [--starred]
tranrss article read <id>        # 纯文本输出
tranrss article translate <id>
tranrss article summarize <id>
tranrss article star <id>
tranrss article mark-read <id>

# 任务队列
tranrss job list [--status pending|running|done|failed]
tranrss job retry <id>
tranrss job clear

# API 配置
tranrss api list
tranrss api add <name> <type> <url> <key>
tranrss api delete <id>

# 配置管理
tranrss config show
tranrss config set <key> <value>
tranrss login --server http://... --api-key trss_xxx
```

### 5. TUI 交互界面

标签页式布局，数字键 1-5 切换：

```
┌─ [1:文章] [2:订阅] [3:队列] [4:API] [5:设置] ──────── TranRSS ──┐
│                                                                  │
│  （每个标签页有不同的布局）                                        │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│ 状态栏：操作提示 / 错误信息 / 加载状态                            │
└──────────────────────────────────────────────────────────────────┘
```

**标签 1：文章**（三栏）
- 左：订阅树（按分类折叠，显示未读数）
- 中：文章列表（标题、日期、已读/收藏状态）
- 右：文章内容（HTML → 纯文本渲染，翻译高亮）

**标签 2：订阅**（列表 + 操作）
- 列表：名称、分类、翻译状态、同步时间
- 操作：添加、编辑、删除、同步

**标签 3：队列**（列表）
- 列表：类型、标题、状态、时间
- 操作：重试、清除

**标签 4：API**（列表）
- 列表：名称、类型、模型、用量
- 操作：添加、编辑、删除

**标签 5：设置**（分组）
- 账号、外观、系统功能

**文章内容渲染（HTML → 纯文本）：**
- `<h1-h6>` → 加粗 + 换行
- `<p>` → 段落
- `<a>` → `[文本](链接)`
- `<img>` → `[图片](url)`
- `<ul/ol>` → 列表
- 翻译块 `<em class="trans-text">` → 绿色高亮

**全局快捷键：**
- `1-5` 切换标签
- `q` 退出
- `?` 帮助
- `/` 搜索
- `r` 刷新

### 6. 配置文件

`~/.config/tranrss/config.toml`：
```toml
server = "http://localhost:8000"
api_key = "trss_xxx"

[log]
output = "auto"
level = "info"

[tui]
theme = "dark"
language = "zh"
```

CLI 参数优先级 > 环境变量 > 配置文件 > 默认值

## 依赖变更

### 新增 crate

| crate | 用途 |
|-------|------|
| `clap` | CLI 参数解析（子命令） |
| `tracing-journald` | systemd 日志 |
| `tracing-syslog` | syslog 日志 |
| `tracing-appender` | 文件日志 |
| `ratatui` | TUI 框架 |
| `crossterm` | 终端控制 |
| `toml` | 配置文件 |
| `html2text` | HTML → 纯文本 |

### 移除

| crate | 原因 |
|-------|------|
| `apalis` | 调度逻辑移到系统层 |
| `apalis-sql` | 同上 |
| `apalis-cron` | 同上 |
| `apalis-core` | 同上 |

## 目录结构

```
tranrss/
├── backend/                # 现有后端（改造为库）
│   └── src/
│       ├── lib.rs          # 导出公共 API
│       ├── main.rs         # 移除，改为 tui/main.rs
│       ├── model/
│       ├── route/
│       ├── services/
│       └── utils/
├── tui/                    # 新的统一入口
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # CLI 入口（clap）
│       ├── cli/            # CLI 子命令实现
│       │   ├── mod.rs
│       │   ├── sub.rs
│       │   ├── article.rs
│       │   ├── job.rs
│       │   ├── api.rs
│       │   ├── config.rs
│       │   └── setup.rs
│       ├── tui/            # TUI 交互界面
│       │   ├── mod.rs
│       │   ├── app.rs
│       │   ├── ui.rs
│       │   └── views/
│       ├── serve.rs        # 统一服务启动
│       ├── cron.rs         # 定时任务执行
│       ├── scheduler.rs    # 系统调度器检测/安装
│       ├── logging.rs      # 日志系统
│       ├── config.rs       # 配置管理
│       └── api_client.rs   # API 客户端
├── frontend/               # 现有前端（不变）
└── Cargo.toml              # workspace（可选）
```

## 实施顺序

1. **改造 backend 为库** — `lib.rs` 导出，移除 `main.rs`
2. **创建 tui crate** — CLI 框架 + 配置 + 日志
3. **实现 CLI 子命令** — sub, article, job, api, config
4. **实现 TUI 交互界面** — 标签页、文章渲染
5. **实现系统调度** — setup 命令生成 systemd/cron 配置
6. **实现 serve 命令** — 统一启动 Web + 定时 + TUI
7. **移除 Apalis** — 清理依赖

## 与现有系统的关系

- **Web 前端** — 不变，仍然通过 `tranrss serve` 启动
- **数据库** — 不变，SQLite 文件共享
- **API** — 不变，TUI/CLI 通过 HTTP 调用后端 API
- **Docker** — 需要更新 Dockerfile，用 `tranrss serve` 替代直接运行 backend
