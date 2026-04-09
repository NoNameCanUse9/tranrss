#!/bin/sh

# 🕒 架构优化：内置 Cron 触发器
# --------------------------------------------------
# 为了实现 FreshRSS 风格的“零闲置功耗”，我们默认禁用 Tokio 内部的高频定时器。
# 取而代之的是，在容器内启动 Alpine 自带的轻量级 crond。

echo "🔧 正在配置内部 Crontab..."

# 每 15 分钟通过 curl 访问内部接口触发一次全量刷新检查
# 注意：此接口仅供内部调用，不涉及 JWT 认证
echo "*/15 * * * * curl -X POST http://127.0.0.1:8000/api/internal/trigger_refresh_all > /dev/null 2>&1" > /etc/crontabs/root

# 启动 cron 守护进程 (-b: 后台运行, -L 0: 关闭日志以减少 I/O)
crond -b -L 0
echo "✅ 内部 Crond 已启动 (周期: 15分钟)"

# 🚀 启动 TranRSS 主进程
# --------------------------------------------------
# 设置默认开启禁用内部定时器的变量（除非用户显式覆盖）
if [ -z "$DISABLE_INTERNAL_CRON" ]; then
    export DISABLE_INTERNAL_CRON=1
fi

echo "🚀 启动 TranRSS Backend..."
# 使用 exec 确保主进程能正确接收 Docker 停止信号 (SIGTERM)
exec ./tranrss
