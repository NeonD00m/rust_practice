use crate::core::types::Symbol;

pub struct ColumnFormatter;

impl ColumnFormatter {
    /// Formats SLOC metric into [  120 loc] or [1,425 loc]
    pub fn format_loc(sloc: usize) -> String {
        let formatted_num = format_with_commas(sloc);
        format!("[{:>6} loc]", formatted_num)
    }

    /// Formats extracted symbols into: class EngineCore, struct EngineState
    pub fn format_symbols(symbols: &[Symbol]) -> String {
        if symbols.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        for sym in symbols.iter().take(4) {
            parts.push(format!("{} {}", sym.kind.badge(), sym.name));
        }

        if symbols.len() > 4 {
            parts.push(format!("+{} more", symbols.len() - 4));
        }

        parts.join(", ")
    }

    /// Aligns tree item with columns
    pub fn format_tree_line(
        branch_and_name: &str,
        tree_width: usize,
        loc_str: Option<&str>,
        symbols_str: Option<&str>,
    ) -> String {
        let mut out = String::new();
        let current_len = branch_and_name.chars().count();

        out.push_str(branch_and_name);

        if loc_str.is_none() && symbols_str.is_none() {
            return out;
        }

        let target_col = tree_width.max(current_len + 2);
        let padding = " ".repeat(target_col.saturating_sub(current_len));
        out.push_str(&padding);

        if let Some(loc) = loc_str {
            out.push_str(loc);
            out.push(' ');
        }

        if let Some(sym) = symbols_str {
            if !sym.is_empty() {
                out.push(' ');
                out.push_str(sym);
            }
        }

        out
    }
}

pub fn format_with_commas(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let len = s.len();

    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}
