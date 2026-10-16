use chrono::{Datelike, Duration, Local};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const DAY_LABELS: [&str; 7] = ["Sun ", "Mon ", "Tue ", "Wed ", "Thu ", "Fri ", "Sat "];
const MONTH_NAMES: [&str; 13] = [
    "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// ANSI 256-color codes for contribution levels
// Level 0: Empty ("  ")
// Level 1: Dark green (22)
// Level 2: Medium-dark green (28)
// Level 3: Medium green (34)
// Level 4: Bright green (40)
const LEVEL_COLORS: [&str; 5] = [
    "  ",
    "\x1b[48;5;22m  \x1b[0m",
    "\x1b[48;5;28m  \x1b[0m",
    "\x1b[48;5;34m  \x1b[0m",
    "\x1b[48;5;40m  \x1b[0m",
];

fn get_cell_str(level: u8) -> &'static str {
    if env::var_os("NO_COLOR").is_some() {
        match level {
            0 => "  ",
            1 => ". ",
            2 => ": ",
            3 => "* ",
            _ => "# ",
        }
    } else {
        match level {
            1 => LEVEL_COLORS[1],
            2 => LEVEL_COLORS[2],
            3 => LEVEL_COLORS[3],
            4 => LEVEL_COLORS[4],
            _ => LEVEL_COLORS[0],
        }
    }
}

struct DayData {
    date: String,
    level: u8,
}

struct WeekData {
    days: [Option<DayData>; 7],
}

struct ContributionGrid {
    title_label: &'static str,
    title_value: String,
    total_count: usize,
    unit: &'static str,
    weeks: Vec<WeekData>,
}

fn render_grid(grid: &ContributionGrid) {
    println!("{}: {}", grid.title_label, grid.title_value);
    println!(
        "Total {} in the last 52 weeks: {}",
        grid.unit, grid.total_count
    );
    println!();

    let cols = grid.weeks.len();
    if cols == 0 {
        println!("No contribution data found.");
        return;
    }

    // 1. Build Month Header
    let mut header_chars = vec![' '; cols * 2];
    let mut prev_month = 0;

    for c in 0..cols {
        let first_day = (0..7).find_map(|r| grid.weeks[c].days[r].as_ref().map(|d| &d.date));

        if let Some(date_str) = first_day
            && date_str.len() >= 7
            && let Ok(mon) = date_str[5..7].parse::<usize>()
            && mon != prev_month
            && (1..=12).contains(&mon)
        {
            let lbl = MONTH_NAMES[mon];
            let pos = c * 2;
            for (k, ch) in lbl.chars().enumerate() {
                if pos + k < header_chars.len() {
                    header_chars[pos + k] = ch;
                }
            }
            prev_month = mon;
        }
    }

    let header_str: String = header_chars.into_iter().collect();
    println!("    {}", header_str);

    // 2. Render Calendar Grid Rows
    for (row, label) in DAY_LABELS.iter().enumerate() {
        let mut line = String::with_capacity(4 + cols * 24);
        line.push_str(label);

        for col in 0..cols {
            if let Some(day) = &grid.weeks[col].days[row] {
                line.push_str(get_cell_str(day.level));
            } else {
                line.push_str("  ");
            }
        }
        println!("{}", line);
    }
    println!();
}

// --- GitHub GraphQL Types ---

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Deserialize)]
struct GraphQLData {
    viewer: Option<UserContributions>,
    user: Option<UserContributions>,
}

#[derive(Deserialize)]
struct UserContributions {
    login: String,
    #[serde(rename = "contributionsCollection")]
    contributions_collection: ContributionsCollection,
}

#[derive(Deserialize)]
struct ContributionsCollection {
    #[serde(rename = "contributionCalendar")]
    contribution_calendar: ContributionCalendar,
}

#[derive(Deserialize)]
struct ContributionCalendar {
    #[serde(rename = "totalContributions")]
    total_contributions: usize,
    weeks: Vec<CalendarWeek>,
}

#[derive(Deserialize)]
struct CalendarWeek {
    #[serde(rename = "contributionDays")]
    contribution_days: Vec<ContributionDay>,
}

#[derive(Deserialize)]
struct ContributionDay {
    date: String,
    #[serde(rename = "contributionCount")]
    contribution_count: usize,
    #[serde(rename = "contributionLevel")]
    contribution_level: String,
    weekday: usize,
}

fn fetch_github_contributions(user: Option<&str>) -> Result<ContributionGrid, String> {
    let mut cmd = Command::new("gh");
    cmd.arg("api").arg("graphql");

    match user {
        Some(username) => {
            cmd.arg("-f").arg(
                "query=query($user: String!) { user(login: $user) { login contributionsCollection { contributionCalendar { totalContributions weeks { contributionDays { date contributionCount contributionLevel weekday } } } } } }",
            );
            cmd.arg("-F").arg(format!("user={}", username));
        }
        None => {
            cmd.arg("-f").arg(
                "query=query { viewer { login contributionsCollection { contributionCalendar { totalContributions weeks { contributionDays { date contributionCount contributionLevel weekday } } } } } }",
            );
        }
    }

    let output = cmd.output().map_err(|e| {
        format!(
            "Failed to run GitHub CLI (`gh`): {}.\nPlease make sure `gh` is installed and authenticated (`gh auth login`).",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GitHub API error:\n{}", stderr.trim()));
    }

    let resp: GraphQLResponse = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

    if let Some(errors) = resp.errors
        && let Some(first) = errors.first()
    {
        return Err(format!("GraphQL error: {}", first.message));
    }

    let user_data = resp
        .data
        .and_then(|d| d.viewer.or(d.user))
        .ok_or_else(|| "Could not retrieve user contribution data.".to_string())?;

    let calendar = user_data.contributions_collection.contribution_calendar;

    let mut weeks = Vec::with_capacity(calendar.weeks.len());
    for week in calendar.weeks {
        let mut days: [Option<DayData>; 7] = [None, None, None, None, None, None, None];
        for day in week.contribution_days {
            if day.weekday < 7 {
                let level = match day.contribution_level.as_str() {
                    "FIRST_QUARTILE" => 1,
                    "SECOND_QUARTILE" => 2,
                    "THIRD_QUARTILE" => 3,
                    "FOURTH_QUARTILE" => 4,
                    _ => {
                        if day.contribution_count > 0 {
                            1
                        } else {
                            0
                        }
                    }
                };
                days[day.weekday] = Some(DayData {
                    date: day.date,
                    level,
                });
            }
        }
        weeks.push(WeekData { days });
    }

    Ok(ContributionGrid {
        title_label: "User",
        title_value: user_data.login,
        total_count: calendar.total_contributions,
        unit: "contributions",
        weeks,
    })
}

// --- Local Git Repository Mode ---

fn calculate_repo_level(count: usize, max_count: usize) -> u8 {
    if count == 0 {
        return 0;
    }
    if max_count <= 4 {
        return count.min(4) as u8;
    }
    let q1 = (max_count as f64 * 0.25).ceil() as usize;
    let q2 = (max_count as f64 * 0.50).ceil() as usize;
    let q3 = (max_count as f64 * 0.75).ceil() as usize;
    if count <= q1 {
        1
    } else if count <= q2 {
        2
    } else if count <= q3 {
        3
    } else {
        4
    }
}

fn fetch_git_repo_contributions(repo_path: &Path) -> Result<ContributionGrid, String> {
    if !repo_path.exists() {
        return Err(format!("Path '{}' does not exist.", repo_path.display()));
    }

    let today = Local::now().date_naive();
    let dow = today.weekday().num_days_from_sunday() as usize; // 0 = Sun, 6 = Sat
    let total_weeks = 52;
    let total_days = total_weeks * 7 + dow;
    let start_date = today - Duration::days(total_days as i64);
    let since_str = start_date.format("%Y-%m-%d").to_string();

    let output = Command::new("git")
        .current_dir(repo_path)
        .args([
            "log",
            "--all",
            &format!("--since={}", since_str),
            "--date=short",
            "--format=%ad",
        ])
        .output()
        .map_err(|e| format!("Failed to run `git`: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Git error in '{}':\n{}",
            repo_path.display(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total_commits = 0;
    let mut max_count = 0;

    for line in stdout.lines() {
        let d = line.trim();
        if !d.is_empty() {
            let entry = counts.entry(d.to_string()).or_insert(0);
            *entry += 1;
            total_commits += 1;
            if *entry > max_count {
                max_count = *entry;
            }
        }
    }

    let num_cols = total_weeks + 1;
    let mut weeks = Vec::with_capacity(num_cols);

    for c in 0..num_cols {
        let mut days: [Option<DayData>; 7] = [None, None, None, None, None, None, None];
        for (r, day_slot) in days.iter_mut().enumerate() {
            let day_offset = c * 7 + r;
            if day_offset <= total_days {
                let date = start_date + Duration::days(day_offset as i64);
                let date_str = date.format("%Y-%m-%d").to_string();
                let count = counts.get(&date_str).copied().unwrap_or(0);
                let level = calculate_repo_level(count, max_count);
                *day_slot = Some(DayData {
                    date: date_str,
                    level,
                });
            }
        }
        weeks.push(WeekData { days });
    }

    let repo_name = repo_path
        .canonicalize()
        .ok()
        .and_then(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| repo_path.display().to_string());

    Ok(ContributionGrid {
        title_label: "Repository",
        title_value: repo_name,
        total_count: total_commits,
        unit: "commits",
        weeks,
    })
}

// --- CLI Parsing ---

enum Mode {
    GitHub(Option<String>),
    Repo(PathBuf),
}

fn print_help() {
    println!(
        r#"butternut - fast, zero-bloat terminal contribution graph

USAGE:
    butternut [OPTIONS] [PATH | USERNAME]

ARGS:
    <PATH>        Path to a local git repository
    <USERNAME>    GitHub username to display

OPTIONS:
    -p, --path <PATH>     Show contributions for a local git repository
    -u, --user <USER>     Show contributions for a GitHub user
    -h, --help            Print help information
    -v, --version         Print version information

DEFAULT:
    If no arguments are provided, butternut displays the GitHub
    contribution calendar for the currently authenticated `gh` user."#
    );
}

fn parse_args() -> Result<Mode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Ok(Mode::GitHub(None));
    }

    let mut i = 0;
    let mut explicit_path: Option<PathBuf> = None;
    let mut explicit_user: Option<String> = None;
    let mut positional: Option<String> = None;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-v" | "--version" => {
                println!("butternut {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-p" | "--path" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --path".to_string());
                }
                explicit_path = Some(PathBuf::from(&args[i]));
            }
            "-u" | "--user" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --user".to_string());
                }
                explicit_user = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}", arg));
            }
            _ => {
                if positional.is_none() {
                    positional = Some(args[i].clone());
                } else {
                    return Err(format!("Unexpected argument: {}", args[i]));
                }
            }
        }
        i += 1;
    }

    if let Some(p) = explicit_path {
        return Ok(Mode::Repo(p));
    }

    if let Some(u) = explicit_user {
        return Ok(Mode::GitHub(Some(u)));
    }

    if let Some(pos) = positional {
        let path = PathBuf::from(&pos);
        if path.exists() || pos.contains('/') || pos.contains('\\') || pos == "." || pos == ".." {
            Ok(Mode::Repo(path))
        } else {
            Ok(Mode::GitHub(Some(pos)))
        }
    } else {
        Ok(Mode::GitHub(None))
    }
}

fn main() {
    let mode = match parse_args() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Run `butternut --help` for usage.");
            std::process::exit(1);
        }
    };

    let result = match mode {
        Mode::GitHub(user) => fetch_github_contributions(user.as_deref()),
        Mode::Repo(path) => fetch_git_repo_contributions(&path),
    };

    match result {
        Ok(grid) => render_grid(&grid),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}
