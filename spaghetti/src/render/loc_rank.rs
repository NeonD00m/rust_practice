use std::collections::BTreeMap;
use crate::core::config::Config;
use crate::core::types::FileNode;
use super::columns::format_with_commas;

pub fn render_loc_rank(
    files: &[FileNode],
    flat: bool,
    outline: bool,
    limit: Option<usize>,
    _config: &Config,
) {
    if files.is_empty() {
        println!("No code files found.");
        return;
    }

    if flat {
        render_flat_loc(files, outline, limit);
    } else {
        render_grouped_loc(files, outline, limit);
    }
}

fn render_grouped_loc(files: &[FileNode], outline: bool, limit: Option<usize>) {
    let mut by_lang: BTreeMap<&'static str, Vec<&FileNode>> = BTreeMap::new();
    let mut total_sloc = 0;
    let mut total_types = 0;

    for file in files {
        by_lang
            .entry(file.language.name())
            .or_default()
            .push(file);
        total_sloc += file.metrics.sloc;
        total_types += file.symbols.len();
    }

    // Sort languages by total SLOC descending
    let mut lang_groups: Vec<(&'static str, Vec<&FileNode>)> = by_lang.into_iter().collect();
    lang_groups.sort_by(|a, b| {
        let sum_b: usize = b.1.iter().map(|f| f.metrics.sloc).sum();
        let sum_a: usize = a.1.iter().map(|f| f.metrics.sloc).sum();
        sum_b.cmp(&sum_a)
    });

    for (lang_name, mut group_files) in lang_groups {
        group_files.sort_by(|a, b| b.metrics.sloc.cmp(&a.metrics.sloc));
        let group_sloc: usize = group_files.iter().map(|f| f.metrics.sloc).sum();

        println!(
            "{} ({} files, {} loc)",
            lang_name.to_uppercase(),
            group_files.len(),
            format_with_commas(group_sloc)
        );

        let take_count = limit.unwrap_or(group_files.len());
        for file in group_files.into_iter().take(take_count) {
            let path_str = file.rel_path.to_string_lossy().replace('\\', "/");
            println!("    {:<35} {:>7} loc", path_str, format_with_commas(file.metrics.sloc));

            if outline && !file.symbols.is_empty() {
                for sym in &file.symbols {
                    println!("        • [{:<6}] {}", sym.kind.badge(), sym.name);
                }
            }
        }
        println!();
    }

    if outline {
        println!("Total: {} files | {} types defined", files.len(), total_types);
    } else {
        println!("Total: {} code files | {} SLOC", files.len(), format_with_commas(total_sloc));
    }
}

fn render_flat_loc(files: &[FileNode], outline: bool, limit: Option<usize>) {
    let mut sorted: Vec<&FileNode> = files.iter().collect();
    sorted.sort_by(|a, b| b.metrics.sloc.cmp(&a.metrics.sloc));

    println!("RANK   {:<35} {:<10} {:>8}", "FILE", "LANG", "SLOC");

    let count = limit.unwrap_or(sorted.len());
    for (idx, file) in sorted.into_iter().take(count).enumerate() {
        let rank = idx + 1;
        let path_str = file.rel_path.to_string_lossy().replace('\\', "/");
        println!(
            "{:>4}.   {:<35} {:<10} {:>8}",
            rank,
            path_str,
            file.language.name(),
            format_with_commas(file.metrics.sloc)
        );

        if outline && !file.symbols.is_empty() {
            for sym in &file.symbols {
                println!("           • [{:<6}] {}", sym.kind.badge(), sym.name);
            }
        }
    }
}
