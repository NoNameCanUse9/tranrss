# TranRSS

TranRSS 是一款基于 AI 驱动的现代 RSS 阅读器。它不仅具备传统的订阅同步功能，更深度集成了 AI 翻译与摘要能力，旨在帮助用户跨越语言障碍，高效获取全球信息。

## 🌟 核心特性

- **AI 智能翻译**：支持 OpenAI 及其兼容 API（如 DeepSeek, Groq 等），可对文章进行高保真全文翻译。
- **AI 核心摘要**：一键生成文章要点，快速筛选有价值内容。
- **GReader API 兼容**：完美支持 Google Reader 协议，可作为各类移动端（如 NetNewsWire, Reeder 等）的后端服务。
- **Token 使用量统计**：透明化展示 AI 接口的使用量（Prompt/Completion），帮助掌控成本。
- **极致自定义样式**：支持用户自定义 CSS，随心所欲定制翻译文本的展示外观。
- **多架构支持**：原生支持 Docker 部署，适配 `amd64` 与 `arm64` 架构（支持树莓派等设备）。

## 🚀 快速开始

### 使用 Docker (推荐方式一)

```bash
docker run -d \
  --name tranrss \
  -p 8000:8000 \
  --restart always \
  -v ./data:/app/data \
  -e DATABASE_URL=sqlite:/app/data/data.database \
  -e API_ENCRYPTION_KEY=你的加密密钥 \
  ghcr.io/nonamecanuse9/tranrss:latest
```

### 使用 Docker Compose (推荐方式二)

项目根目录下已提供 `docker-compose.yml`，你可以直接运行：

```bash
docker compose up -d
```

> [!IMPORTANT]
> - **持久化映射**：推荐直接映射 `./data` 或 `/app/data` 目录以兼容 SQLite 的 WAL 模式临时文件。
> - **端口说明**：Docker 镜像已开启 `embed-frontend`，访问 **8000** 即可完成全部操作（前端单独开发的调试端口为 8001）。

### 环境变量

| 变量名 | 说明 | 默认值 |
| :--- | :--- | :--- |
| `DATABASE_URL` | SQLite 数据库路径 | `sqlite:/app/data/data.database` |
| `API_ENCRYPTION_KEY` | API 密钥加密 Key | (内置默认) |
| `JWT_SECRET` | JWT 鉴权私钥 | (自动生成) |

## 🛠️ 技术栈

### 后端 (Rust)
- **Web 框架**: [Axum](https://github.com/tokio-rs/axum) (高性能异步框架)
- **异步运行时**: [Tokio](https://tokio.rs/)
- **数据库**: [SQLx](https://github.com/launchbadge/sqlx) + SQLite
- **任务队列**: [Apalis](https://github.com/geofreyabbott/apalis) (通过 SQLite 实现的持久化作业队列)
- **安全**: Bcrypt (密码哈希), Magic-crypt (密钥加密)

### 前端 (Vue 3)
- **框架**: Vue 3.5 (Composition API)
- **UI 组件库**: Vuetify 4.0-beta
- **构建工具**: Vite 8.0
- **数据可视化**: ECharts
- **国际化**: vue-i18n

## 🔧 开发调试

```bash
# 进入后端目录
cd backend
pnpm dev

# 进入前端目录
cd frontend
pnpm dev
```

## 📄 开源协议

本项目采用 MIT 协议。
