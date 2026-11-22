//! 批量下载链接解析与 IDM 经典通配符范围展开引擎

use std::collections::HashSet;

/// 解析多行输入并递归展开所有 `[start-end]` 通配符，返回去重后的最终下载 URL 列表
///
/// 支持语法：
/// - 普通多行链接（自动忽略空行与 `#` 注释行）
/// - 数字范围通配符：`https://example.com/file_[01-05].zip`（自动保留前导零宽度）
/// - 字母范围通配符：`https://example.com/part_[a-d].rar`
/// - 多重组合通配符：`https://example.com/season_[01-02]/ep_[01-03].mp4`
pub fn expand_batch_urls(raw_input: &str) -> Vec<String> {
    const MAX_TOTAL_URLS: usize = 1000;
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for raw_line in raw_input.lines() {
        for segment in raw_line.split(|c| c == ';' || c == ',') {
            let trimmed = segment.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut queue = vec![trimmed.to_string()];
            while let Some(current) = queue.pop() {
                let expanded = expand_single_pattern(&current);
                if expanded.len() == 1 && expanded[0] == current {
                    if seen.insert(current.clone()) {
                        results.push(current);
                        if results.len() >= MAX_TOTAL_URLS {
                            return results;
                        }
                    }
                } else {
                    // 逆序压栈以保持原始展开顺序
                    for item in expanded.into_iter().rev() {
                        queue.push(item);
                    }
                }
            }
        }
    }
    results
}

/// 展开单条 URL 中的第一个 `[start-end]` 通配符（若无合法通配符则原样返回）
fn expand_single_pattern(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('[') else {
        return vec![pattern.to_string()];
    };
    let Some(rel_close) = pattern[open + 1..].find(']') else {
        return vec![pattern.to_string()];
    };
    let close = open + 1 + rel_close;
    let inner = &pattern[open + 1..close];
    let Some((start_str, end_str)) = inner.split_once('-') else {
        return vec![pattern.to_string()];
    };

    let prefix = &pattern[..open];
    let suffix = &pattern[close + 1..];

    // 1. 尝试按数字范围展开（自动保留前导零宽度，上限单次 500 条防止误填超大范围）
    if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
        if start <= end && (end - start) <= 500 {
            let width = if start_str.starts_with('0') && start_str.len() > 1 {
                start_str.len()
            } else {
                0
            };
            return (start..=end)
                .map(|n| {
                    if width > 0 {
                        format!("{prefix}{n:0width$}{suffix}")
                    } else {
                        format!("{prefix}{n}{suffix}")
                    }
                })
                .collect();
        }
    }

    // 2. 尝试按单字母范围展开（如 [a-d] 或 [A-D]）
    let mut sc = start_str.chars();
    let mut ec = end_str.chars();
    if let (Some(a), None, Some(b), None) = (sc.next(), sc.next(), ec.next(), ec.next()) {
        if a.is_ascii_alphabetic() && b.is_ascii_alphabetic() && a <= b {
            return (a..=b).map(|ch| format!("{prefix}{ch}{suffix}")).collect();
        }
    }

    vec![pattern.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_zero_padded_numeric_range() {
        let urls = expand_batch_urls("https://example.com/ep_[01-03].mp4");
        assert_eq!(
            urls,
            vec![
                "https://example.com/ep_01.mp4",
                "https://example.com/ep_02.mp4",
                "https://example.com/ep_03.mp4",
            ]
        );
    }

    #[test]
    fn test_expand_alpha_range_and_dedup() {
        let input = "\n# comment\nhttps://example.com/p_[a-c].zip\nhttps://example.com/p_b.zip\n";
        let urls = expand_batch_urls(input);
        assert_eq!(
            urls,
            vec![
                "https://example.com/p_a.zip",
                "https://example.com/p_b.zip",
                "https://example.com/p_c.zip",
            ]
        );
    }

    #[test]
    fn test_invalid_pattern_keeps_original() {
        let urls = expand_batch_urls("https://example.com/file_[invalid].zip");
        assert_eq!(urls, vec!["https://example.com/file_[invalid].zip"]);
    }

    #[test]
    fn test_expand_multi_wildcards() {
        let urls = expand_batch_urls("https://example.com/s[1-2]/ep_[01-02]_[a-b].mp4");
        assert_eq!(
            urls,
            vec![
                "https://example.com/s1/ep_01_a.mp4",
                "https://example.com/s1/ep_01_b.mp4",
                "https://example.com/s1/ep_02_a.mp4",
                "https://example.com/s1/ep_02_b.mp4",
                "https://example.com/s2/ep_01_a.mp4",
                "https://example.com/s2/ep_01_b.mp4",
                "https://example.com/s2/ep_02_a.mp4",
                "https://example.com/s2/ep_02_b.mp4",
            ]
        );
    }
}
