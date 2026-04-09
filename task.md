Choken，从这份新的 perf 报告来看，之前的优化已经立竿见影。原本占据统治地位的内存分配风暴（malloc 导致的连锁缺页异常）已经大幅消退。

但正如报告所示，瓶颈发生了转移。现在导致 CPU 空转的核心元凶是 futex（线程抢锁与休眠唤醒调度），这部分占了总开销的 30% 以上，主要集中在 tokio-runtime-w、sqlx-sqlite-wor 以及 futures-timer 这三个线程上。同时，SQLite 线程中还残留了大约 6% 的 mmap/munmap 开销。

要彻底对齐甚至超越 FreshRSS 这种“闲时绝对 0 CPU”的资源占用，我们还有最后几个深水区的优化空间：

1. 架构级降维：用 Systemd/Cron 替代 Tokio 内部定时器
报告中 futures-timer 占用了约 5% 的 CPU。这说明你的代码中大概率跑着 tokio::time::interval 或者大量的异步 sleep 来轮询 RSS 源。

FreshRSS 的省资源秘诀：它依靠系统的 Cron 唤醒，工作完进程直接死掉。而你的 Tokio 运行时为了维护这些内部定时器，底层调度器一直在高频触发 futex 唤醒。

终极优化方案：删掉 Rust 代码里的后台无限轮询任务。给你的应用暴露一个仅限本地访问的接口（例如 POST /api/internal/trigger_update）。然后利用你熟悉的 Linux 环境，写一个简单的 systemd.timer 或在 OpenWrt 里配一条 crontab，每隔 15 分钟通过 curl 触发一次更新。

效果：在没有请求的 14分59秒 里，Tokio 调度器将处于绝对沉睡状态，futures-timer 的唤醒开销直接归零。

2. 数据库极致减负：事务批处理 (Transaction Batching)
报告里 sqlx-sqlite-wor 依然有高达 17% 的 futex 锁等待。这通常发生在你解析完 XML 后，将文章一条一条地 INSERT 进数据库。每一次单独的 execute().await 都会触发 SQLite 的文件锁获取和释放。

优化方案：将单个订阅源的更新打包进一个事务（Transaction）中。

Rust
// 绝对不要这样做：
// for item in items { sqlx::query("INSERT...").execute(&pool).await?; }

// 应该这样做：
let mut tx = pool.begin().await?;
for item in items {
    sqlx::query("INSERT...") // 绑定参数
        .execute(&mut *tx)
        .await?;
}
tx.commit().await?;
效果：SQLite 在一个事务里写 100 条记录的开销几乎等同于写 1 条。这能让 sqlx-sqlite-wor 线程的锁竞争断崖式下跌。

3. 内存系统调用的“最后一公里”：Jemalloc 衰减控制
报告中依然残留了 __x64_sys_munmap (3.61%) 和 do_mmap (2.67%)。这是因为 Jemalloc 虽然高效，但它默认会相对积极地把暂时不用的内存“归还”给操作系统（触发 munmap），下次要用时再向系统要（触发 mmap）。

优化方案：在启动 tranrss 时，通过环境变量调整 Jemalloc 的脏页回收策略，让它把内存多“捂”在应用层一会儿。

Bash
export MALLOC_CONF="background_thread:true,dirty_decay_ms:30000,muzzy_decay_ms:30000"
./tranrss
效果：开启后台线程专门清理碎片，并将内存交还给系统的延迟设为 30 秒。这会稍微增加一点平时的常驻内存（RSS），但彻底抹除掉因解析 XML 引起的最后一丝 mmap/munmap CPU 损耗。

4. 优化 SQLite 的 I/O 映射 (Mmap Size)
最后，SQLite 自身在读取文件时也会调用系统 mmap。通过在数据库连接池初始化时执行一条 PRAGMA 指令，可以让 SQLite 扩大内存映射的尺寸，避免频繁的换页操作：

SQL
PRAGMA mmap_size=268435456; -- 设置为 256MB
总结：
完成上述调整后（尤其是剥离内部定时器改用系统级触发，以及事务批处理），你的 tranrss 将真正蜕变为一个极其轻量的网络服务。平时占用几 MB 到十几 MB 的极小内存，CPU 稳定在 0.0%，只有在系统层面触发抓取的那几秒钟才会满载运转，从而在资源占用上完美战胜 FreshRSS。