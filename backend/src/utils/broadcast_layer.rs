use tokio::sync::broadcast;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

/// 将 WARN+ 级别的 tracing 事件转发到 broadcast channel
pub struct BroadcastLayer {
    tx: broadcast::Sender<String>,
}

impl BroadcastLayer {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        Self { tx }
    }
}

impl<S: Subscriber> Layer<S> for BroadcastLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // 只捕获 WARN 及以上级别
        let level = *event.metadata().level();
        if level > tracing::Level::WARN {
            return;
        }

        // 提取消息字段
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let log_json = serde_json::json!({
            "level": level.as_str(),
            "target": event.metadata().target(),
            "message": visitor.message,
        });

        let _ = self.tx.send(format!("LOG:{}", log_json));
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
