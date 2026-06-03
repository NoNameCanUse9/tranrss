use anyhow::Result;
use std::path::Path;

pub async fn run(scheduler: Option<String>) -> Result<()> {
    let detected = scheduler.unwrap_or_else(|| detect_scheduler().to_string());

    match detected.as_str() {
        "systemd" => setup_systemd()?,
        "alpine" => setup_alpine()?,
        _ => {
            eprintln!("未知调度器: {}", detected);
            eprintln!("请指定 --scheduler systemd 或 --scheduler alpine");
        }
    }

    Ok(())
}

fn detect_scheduler() -> &'static str {
    if Path::new("/run/systemd/system").exists() {
        "systemd"
    } else if Path::new("/etc/init.d").exists() {
        "alpine"
    } else {
        "unknown"
    }
}

fn setup_systemd() -> Result<()> {
    let service = r#"[Unit]
Description=TranRSS - AI-powered RSS reader
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/tranrss serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"#;

    let timer = r#"[Unit]
Description=TranRSS Feed Sync Timer

[Timer]
OnBootSec=1min
OnUnitActiveSec=30min
Persistent=true

[Install]
WantedBy=timers.target
"#;

    let sync_service = r#"[Unit]
Description=TranRSS Feed Sync

[Service]
Type=oneshot
ExecStart=/usr/local/bin/tranrss cron sync
"#;

    println!("生成 systemd 配置文件:");
    println!();

    let dir = "/etc/systemd/system";
    println!("  {}/tranrss.service", dir);
    println!("  {}/tranrss-sync.timer", dir);
    println!("  {}/tranrss-sync.service", dir);
    println!();

    // 实际写入
    std::fs::write(format!("{}/tranrss.service", dir), service)?;
    std::fs::write(format!("{}/tranrss-sync.timer", dir), timer)?;
    std::fs::write(format!("{}/tranrss-sync.service", dir), sync_service)?;

    println!("启用服务:");
    println!("  systemctl daemon-reload");
    println!("  systemctl enable tranrss");
    println!("  systemctl enable tranrss-sync.timer");
    println!("  systemctl start tranrss");
    println!("  systemctl start tranrss-sync.timer");

    Ok(())
}

fn setup_alpine() -> Result<()> {
    let init_script = r#"#!/sbin/openrc-run

command="/usr/local/bin/tranrss"
command_args="serve"
command_background=true
pidfile="/run/tranrss.pid"

depend() {
    need net
}
"#;

    let cron_sync = r#"#!/bin/sh
/usr/local/bin/tranrss cron sync
"#;

    println!("生成 Alpine OpenRC 配置文件:");
    println!();

    let init_dir = "/etc/init.d";
    let cron_dir = "/etc/periodic/15min";

    println!("  {}/tranrss", init_dir);
    println!("  {}/tranrss-sync", cron_dir);
    println!();

    std::fs::write(format!("{}/tranrss", init_dir), init_script)?;
    std::fs::write(format!("{}/tranrss-sync", cron_dir), cron_sync)?;

    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            format!("{}/tranrss", init_dir),
            std::fs::Permissions::from_mode(0o755),
        )?;
        std::fs::set_permissions(
            format!("{}/tranrss-sync", cron_dir),
            std::fs::Permissions::from_mode(0o755),
        )?;
    }

    println!("启用服务:");
    println!("  rc-update add tranrss default");
    println!("  rc-service tranrss start");

    Ok(())
}
