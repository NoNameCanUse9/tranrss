/// 上游错误文本没有长度上限：HTTP 失败时可能是整页 HTML，任务失败时可能是完整调用栈。
/// 直接返回给前端会让列表把整段错误塞进 DOM，也白占数据库空间。
/// 这里按字符（不是字节）截断，避免在多字节字符中间切开。
pub const MAX_ERROR_TEXT_CHARS: usize = 300;

pub fn truncate_error_text(text: &str) -> String {
    let mut out = String::with_capacity(MAX_ERROR_TEXT_CHARS + 4);
    let mut count = 0usize;

    for ch in text.chars() {
        // 换行/制表在卡片里本就折叠显示，保留会撑高行数
        let ch = if ch.is_control() { ' ' } else { ch };
        if count == MAX_ERROR_TEXT_CHARS {
            out.push('…');
            break;
        }
        out.push(ch);
        count += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_short_messages_intact() {
        assert_eq!(truncate_error_text("connection refused"), "connection refused");
    }

    #[test]
    fn truncates_by_chars_not_bytes() {
        let long = "错".repeat(MAX_ERROR_TEXT_CHARS + 10);
        let out = truncate_error_text(&long);
        assert_eq!(out.chars().count(), MAX_ERROR_TEXT_CHARS + 1);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn collapses_control_characters() {
        assert_eq!(truncate_error_text("line1\nline2\ttab"), "line1 line2 tab");
    }

    #[test]
    fn is_idempotent_on_already_short_text() {
        let once = truncate_error_text("boom");
        assert_eq!(truncate_error_text(&once), once);
    }
}
