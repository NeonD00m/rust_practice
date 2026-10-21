use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::io::IsTerminal;
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

fn is_no_color() -> bool {
    env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal()
}

fn get_cell_str(level: u8) -> &'static str {
    if is_no_color() {
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
    count: usize,
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
    timeframe_desc: String,
}

struct StreakStats {
    current_streak: usize,
    longest_streak: usize,
    today_count: usize,
    is_active_today: bool,
}

fn calculate_streaks(weeks: &[WeekData]) -> StreakStats {
    let mut all_days: Vec<&DayData> = Vec::new();
    for week in weeks {
        for day in week.days.iter().flatten() {
            all_days.push(day);
        }
    }

    if all_days.is_empty() {
        return StreakStats {
            current_streak: 0,
            longest_streak: 0,
            today_count: 0,
            is_active_today: false,
        };
    }

    // Longest streak across all recorded days in the period
    let mut longest_streak = 0;
    let mut current_run = 0;
    for day in &all_days {
        if day.count > 0 {
            current_run += 1;
            if current_run > longest_streak {
                longest_streak = current_run;
            }
        } else {
            current_run = 0;
        }
    }

    let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let today_idx = all_days.iter().position(|d| d.date == today_str);

    let (current_streak, today_count, is_active_today) = match today_idx {
        Some(idx) => {
            let count = all_days[idx].count;
            if count > 0 {
                // Today has contributions -> streak is active today
                let mut streak = 1;
                let mut i = idx;
                while i > 0 {
                    i -= 1;
                    if all_days[i].count > 0 {
                        streak += 1;
                    } else {
                        break;
                    }
                }
                (streak, count, true)
            } else {
                // Today has 0 contributions -> check if streak carried through from yesterday
                let mut streak = 0;
                let mut i = idx;
                while i > 0 {
                    i -= 1;
                    if all_days[i].count > 0 {
                        streak += 1;
                    } else {
                        break;
                    }
                }
                (streak, 0, false)
            }
        }
        None => (0, 0, false),
    };

    StreakStats {
        current_streak,
        longest_streak,
        today_count,
        is_active_today,
    }
}

fn format_streak(stats: &StreakStats) -> String {
    let emoji = if stats.is_active_today {
        "🔥"
    } else {
        "🕯️"
    };

    format!(
        "{} {} days ({} today)",
        emoji, stats.current_streak, stats.today_count
    )
}

fn get_terminal_width() -> Option<usize> {
    if let Ok(cols) = env::var("COLUMNS")
        && let Ok(w) = cols.trim().parse::<usize>()
        && w > 0
    {
        return Some(w);
    }

    #[cfg(windows)]
    {
        #[repr(C)]
        struct Coord {
            x: i16,
            y: i16,
        }
        #[repr(C)]
        struct SmallRect {
            left: i16,
            top: i16,
            right: i16,
            bottom: i16,
        }
        #[repr(C)]
        struct ConsoleScreenBufferInfo {
            size: Coord,
            cursor_pos: Coord,
            attrs: u16,
            window: SmallRect,
            max_window_size: Coord,
        }
        unsafe extern "system" {
            fn GetStdHandle(n_std_handle: u32) -> *mut std::ffi::c_void;
            fn GetConsoleScreenBufferInfo(
                h_console: *mut std::ffi::c_void,
                lp_info: *mut ConsoleScreenBufferInfo,
            ) -> i32;
        }
        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if !handle.is_null() && handle != -1isize as *mut std::ffi::c_void {
                let mut info = std::mem::zeroed::<ConsoleScreenBufferInfo>();
                if GetConsoleScreenBufferInfo(handle, &mut info) != 0 {
                    let w = (info.window.right - info.window.left + 1) as usize;
                    if w > 0 {
                        return Some(w);
                    }
                }
            }
        }
    }

    #[cfg(unix)]
    {
        #[repr(C)]
        struct Winsize {
            ws_row: u16,
            ws_col: u16,
            ws_xpixel: u16,
            ws_ypixel: u16,
        }
        unsafe extern "C" {
            fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
        }
        unsafe {
            let mut ws: Winsize = std::mem::zeroed();
            #[cfg(target_os = "linux")]
            const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
            #[cfg(target_os = "macos")]
            const TIOCGWINSZ: std::ffi::c_ulong = 0x40087468;
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;

            if ioctl(1, TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
                return Some(ws.ws_col as usize);
            }
        }
    }

    None
}

fn fit_weeks_to_terminal(mut weeks: Vec<WeekData>, explicit_weeks: Option<usize>) -> Vec<WeekData> {
    if weeks.is_empty() {
        return weeks;
    }

    let target_weeks = if let Some(w) = explicit_weeks {
        w
    } else if let Some(term_width) = get_terminal_width() {
        // Line layout: "Sun " (4 chars) + (cols * 2 chars)
        // With 1 char safety margin to avoid automatic console edge wrap:
        // term_width - 1 >= 4 + cols * 2  ==> cols <= (term_width - 5) / 2
        let available = term_width.saturating_sub(5);
        let max_cols = available / 2;
        max_cols.max(1)
    } else {
        weeks.len()
    };

    if target_weeks < weeks.len() {
        let drop_count = weeks.len() - target_weeks;
        weeks.drain(0..drop_count);
    }
    weeks
}

fn build_month_header(weeks: &[WeekData]) -> String {
    let cols = weeks.len();
    if cols == 0 {
        return String::new();
    }

    let mut header_chars = vec![' '; cols * 2];
    struct MonthPlacement {
        col: usize,
        month: usize,
    }
    let mut placements: Vec<MonthPlacement> = Vec::new();
    let mut prev_month = 0;

    for c in 0..cols {
        let first_day = (0..7).find_map(|r| weeks[c].days[r].as_ref().map(|d| &d.date));

        if let Some(date_str) = first_day
            && date_str.len() >= 7
            && let Ok(mon) = date_str[5..7].parse::<usize>()
            && mon != prev_month
            && (1..=12).contains(&mon)
        {
            placements.push(MonthPlacement { col: c, month: mon });
            prev_month = mon;
        }
    }

    for (i, p) in placements.iter().enumerate() {
        let pos = p.col * 2;
        // Omit month if it cannot fit all 3 characters before the end of the header
        if pos + 3 > header_chars.len() {
            continue;
        }
        // Omit month if the next month starts so soon that it would overwrite/truncate this label
        if let Some(next_p) = placements.get(i + 1) {
            let next_pos = next_p.col * 2;
            if pos + 3 > next_pos {
                continue;
            }
        }

        let lbl = MONTH_NAMES[p.month];
        for (k, ch) in lbl.chars().enumerate() {
            if pos + k < header_chars.len() {
                header_chars[pos + k] = ch;
            }
        }
    }

    header_chars.into_iter().collect()
}

fn render_grid(mut grid: ContributionGrid, explicit_weeks: Option<usize>) {
    let streak_stats = calculate_streaks(&grid.weeks);
    grid.weeks = fit_weeks_to_terminal(grid.weeks, explicit_weeks);

    println!("{}: {}", grid.title_label, grid.title_value);
    println!(
        "Total {} {}: {}",
        grid.unit, grid.timeframe_desc, grid.total_count
    );
    println!(
        "Streak: {} | Longest: {} days",
        format_streak(&streak_stats),
        streak_stats.longest_streak
    );
    println!();

    let cols = grid.weeks.len();
    if cols == 0 {
        println!("No contribution data found.");
        return;
    }

    // 1. Build Month Header
    let header_str = build_month_header(&grid.weeks);
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

fn fetch_github_contributions(
    user: Option<&str>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    timeframe_desc: String,
) -> Result<ContributionGrid, String> {
    let mut cmd = Command::new("gh");
    cmd.arg("api").arg("graphql");

    let has_dates = start_date.is_some() || end_date.is_some();
    let from_str = start_date.map(|d| d.format("%Y-%m-%dT00:00:00Z").to_string());
    let to_str = end_date.map(|d| d.format("%Y-%m-%dT23:59:59Z").to_string());

    match (user, has_dates) {
        (Some(username), true) => {
            cmd.arg("-f").arg(
                "query=query($user: String!, $from: DateTime, $to: DateTime) { user(login: $user) { login contributionsCollection(from: $from, to: $to) { contributionCalendar { totalContributions weeks { contributionDays { date contributionCount contributionLevel weekday } } } } } }",
            );
            cmd.arg("-F").arg(format!("user={}", username));
            if let Some(from) = from_str {
                cmd.arg("-F").arg(format!("from={}", from));
            }
            if let Some(to) = to_str {
                cmd.arg("-F").arg(format!("to={}", to));
            }
        }
        (Some(username), false) => {
            cmd.arg("-f").arg(
                "query=query($user: String!) { user(login: $user) { login contributionsCollection { contributionCalendar { totalContributions weeks { contributionDays { date contributionCount contributionLevel weekday } } } } } }",
            );
            cmd.arg("-F").arg(format!("user={}", username));
        }
        (None, true) => {
            cmd.arg("-f").arg(
                "query=query($from: DateTime, $to: DateTime) { viewer { login contributionsCollection(from: $from, to: $to) { contributionCalendar { totalContributions weeks { contributionDays { date contributionCount contributionLevel weekday } } } } } }",
            );
            if let Some(from) = from_str {
                cmd.arg("-F").arg(format!("from={}", from));
            }
            if let Some(to) = to_str {
                cmd.arg("-F").arg(format!("to={}", to));
            }
        }
        (None, false) => {
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
                    count: day.contribution_count,
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
        timeframe_desc,
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

fn get_git_config_user(repo_path: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["config", "user.name"])
        .output()
        .map_err(|e| format!("Failed to run `git config user.name`: {}", e))?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }

    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["config", "user.email"])
        .output()
        .map_err(|e| format!("Failed to run `git config user.email`: {}", e))?;

    if output.status.success() {
        let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !email.is_empty() {
            return Ok(email);
        }
    }

    Err("Could not determine `user.name` or `user.email` from git config. Use -u <name> to specify author manually.".to_string())
}

fn fetch_git_repo_contributions(
    repo_path: &Path,
    author: Option<&str>,
    start_date_opt: Option<NaiveDate>,
    end_date_opt: Option<NaiveDate>,
    timeframe_desc: String,
) -> Result<ContributionGrid, String> {
    if !repo_path.exists() {
        return Err(format!("Path '{}' does not exist.", repo_path.display()));
    }

    let today = Local::now().date_naive();
    let (cal_start, cal_end, active_start, active_end) = match (start_date_opt, end_date_opt) {
        (Some(start), Some(end)) => {
            let start_dow = start.weekday().num_days_from_sunday() as usize;
            let cal_start = start - Duration::days(start_dow as i64);
            let end_dow = end.weekday().num_days_from_sunday() as usize;
            let cal_end = end + Duration::days((6 - end_dow) as i64);
            (cal_start, cal_end, start, end)
        }
        (Some(start), None) => {
            let start_dow = start.weekday().num_days_from_sunday() as usize;
            let cal_start = start - Duration::days(start_dow as i64);
            let end_dow = today.weekday().num_days_from_sunday() as usize;
            let cal_end = today + Duration::days((6 - end_dow) as i64);
            (cal_start, cal_end, start, today)
        }
        (None, Some(end)) => {
            let start = end - Duration::days(52 * 7);
            let start_dow = start.weekday().num_days_from_sunday() as usize;
            let cal_start = start - Duration::days(start_dow as i64);
            let end_dow = end.weekday().num_days_from_sunday() as usize;
            let cal_end = end + Duration::days((6 - end_dow) as i64);
            (cal_start, cal_end, start, end)
        }
        (None, None) => {
            let total_weeks = 52;
            let dow = today.weekday().num_days_from_sunday() as usize; // 0 = Sun, 6 = Sat
            let total_days = total_weeks * 7 + dow;
            let start_date = today - Duration::days(total_days as i64);
            (start_date, today, start_date, today)
        }
    };

    let since_str = active_start.format("%Y-%m-%d").to_string();
    let until_str = (active_end + Duration::days(1)).format("%Y-%m-%d").to_string();

    let mut git_cmd = Command::new("git");
    git_cmd
        .current_dir(repo_path)
        .arg("log")
        .arg("--all")
        .arg(format!("--since={}", since_str))
        .arg(format!("--until={}", until_str))
        .arg("--date=short")
        .arg("--format=%ad");

    if let Some(auth) = author {
        git_cmd.arg(format!("--author={}", auth));
    }

    let output = git_cmd
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

    let span_days = (cal_end - cal_start).num_days() as usize;
    let num_cols = (span_days / 7) + 1;
    let mut weeks = Vec::with_capacity(num_cols);

    for c in 0..num_cols {
        let mut days: [Option<DayData>; 7] = [None, None, None, None, None, None, None];
        for (r, day_slot) in days.iter_mut().enumerate() {
            let day_offset = c * 7 + r;
            let current_date = cal_start + Duration::days(day_offset as i64);
            if current_date >= active_start && current_date <= active_end {
                let date_str = current_date.format("%Y-%m-%d").to_string();
                let count = counts.get(&date_str).copied().unwrap_or(0);
                let level = calculate_repo_level(count, max_count);
                *day_slot = Some(DayData {
                    date: date_str,
                    count,
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

    let display_title = if let Some(auth) = author {
        format!("{} (author: {})", repo_name, auth)
    } else {
        repo_name
    };

    Ok(ContributionGrid {
        title_label: "Repository",
        title_value: display_title,
        total_count: total_commits,
        unit: "commits",
        weeks,
        timeframe_desc,
    })
}

// --- CLI Parsing ---

enum Mode {
    GitHub {
        user: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        explicit_weeks: Option<usize>,
        timeframe_desc: String,
    },
    Repo {
        path: PathBuf,
        author: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        explicit_weeks: Option<usize>,
        timeframe_desc: String,
    },
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
    -p, --path <PATH>         Show contributions for a local git repository
    -u, --user <USER>         Show contributions for a GitHub user or filter Git repo author
        --author <AUTHOR>     Alias for --user (filter commits by author in Git repo)
        --me                  Filter local git repository commits by current user (git config user.name)
        --all                 Show contributions from all authors in Git repo (default)
    -y, --year <YYYY>         Show contribution calendar for a specific calendar year
        --since <YYYY-MM-DD>  Start date for contribution timeframe
        --until <YYYY-MM-DD>  End date for contribution timeframe
    -w, --weeks <N>           Explicitly set number of weeks to display
    -h, --help                Print help information
    -v, --version             Print version information

DEFAULT:
    If no arguments are provided, butternut displays the GitHub
    contribution calendar for the currently authenticated `gh` user."#
    );
}

fn parse_iso_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': expected YYYY-MM-DD ({})", s, e))
}

fn parse_args() -> Result<Mode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Ok(Mode::GitHub {
            user: None,
            start_date: None,
            end_date: None,
            explicit_weeks: None,
            timeframe_desc: "in the last 52 weeks".to_string(),
        });
    }

    let mut i = 0;
    let mut explicit_path: Option<PathBuf> = None;
    let mut explicit_user: Option<String> = None;
    let mut explicit_author: Option<String> = None;
    let mut positional: Option<String> = None;
    let mut use_me = false;
    let mut show_all = false;
    let mut year: Option<i32> = None;
    let mut since_date: Option<NaiveDate> = None;
    let mut until_date: Option<NaiveDate> = None;
    let mut explicit_weeks: Option<usize> = None;

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
            "--author" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --author".to_string());
                }
                explicit_author = Some(args[i].clone());
            }
            "--me" => {
                use_me = true;
            }
            "--all" => {
                show_all = true;
            }
            "-y" | "--year" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --year".to_string());
                }
                let yr = args[i]
                    .parse::<i32>()
                    .map_err(|_| format!("Invalid year '{}': must be a 4-digit number", args[i]))?;
                year = Some(yr);
            }
            "--since" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --since".to_string());
                }
                since_date = Some(parse_iso_date(&args[i])?);
            }
            "--until" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --until".to_string());
                }
                until_date = Some(parse_iso_date(&args[i])?);
            }
            "-w" | "--weeks" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --weeks".to_string());
                }
                let w = args[i]
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid weeks '{}': must be a positive integer", args[i]))?;
                explicit_weeks = Some(w);
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

    if let Some(yr) = year {
        if since_date.is_some() || until_date.is_some() {
            return Err("Cannot combine --year with --since or --until".to_string());
        }
        since_date = Some(NaiveDate::from_ymd_opt(yr, 1, 1).ok_or("Invalid year")?);
        until_date = Some(NaiveDate::from_ymd_opt(yr, 12, 31).ok_or("Invalid year")?);
    }

    if let (Some(s), Some(u)) = (since_date, until_date)
        && s > u
    {
        return Err(format!("--since ({}) cannot be after --until ({})", s, u));
    }

    let timeframe_desc = if let Some(yr) = year {
        format!("in {}", yr)
    } else if let (Some(s), Some(u)) = (since_date, until_date) {
        format!("from {} to {}", s, u)
    } else if let Some(s) = since_date {
        format!("since {}", s)
    } else if let Some(u) = until_date {
        format!("until {}", u)
    } else if let Some(w) = explicit_weeks {
        format!("in the last {} weeks", w)
    } else {
        "in the last 52 weeks".to_string()
    };

    if let Some(p) = explicit_path {
        let author = if show_all {
            None
        } else if use_me {
            Some(get_git_config_user(&p)?)
        } else {
            explicit_author.or(explicit_user)
        };
        return Ok(Mode::Repo {
            path: p,
            author,
            start_date: since_date,
            end_date: until_date,
            explicit_weeks,
            timeframe_desc,
        });
    }

    if let Some(pos) = positional {
        let path = PathBuf::from(&pos);
        if path.exists() || pos.contains('/') || pos.contains('\\') || pos == "." || pos == ".." {
            let author = if show_all {
                None
            } else if use_me {
                Some(get_git_config_user(&path)?)
            } else {
                explicit_author.or(explicit_user)
            };
            Ok(Mode::Repo {
                path,
                author,
                start_date: since_date,
                end_date: until_date,
                explicit_weeks,
                timeframe_desc,
            })
        } else {
            // Positional argument is a GitHub username
            Ok(Mode::GitHub {
                user: Some(pos),
                start_date: since_date,
                end_date: until_date,
                explicit_weeks,
                timeframe_desc,
            })
        }
    } else if use_me || explicit_author.is_some() {
        // --me or --author specified without path implies local repository at "."
        let path = PathBuf::from(".");
        let author = if use_me {
            Some(get_git_config_user(&path)?)
        } else {
            explicit_author
        };
        Ok(Mode::Repo {
            path,
            author,
            start_date: since_date,
            end_date: until_date,
            explicit_weeks,
            timeframe_desc,
        })
    } else if let Some(u) = explicit_user {
        Ok(Mode::GitHub {
            user: Some(u),
            start_date: since_date,
            end_date: until_date,
            explicit_weeks,
            timeframe_desc,
        })
    } else {
        Ok(Mode::GitHub {
            user: None,
            start_date: since_date,
            end_date: until_date,
            explicit_weeks,
            timeframe_desc,
        })
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

    let (result, explicit_weeks) = match mode {
        Mode::GitHub {
            user,
            start_date,
            end_date,
            explicit_weeks,
            timeframe_desc,
        } => (
            fetch_github_contributions(user.as_deref(), start_date, end_date, timeframe_desc),
            explicit_weeks,
        ),
        Mode::Repo {
            path,
            author,
            start_date,
            end_date,
            explicit_weeks,
            timeframe_desc,
        } => (
            fetch_git_repo_contributions(
                &path,
                author.as_deref(),
                start_date,
                end_date,
                timeframe_desc,
            ),
            explicit_weeks,
        ),
    };

    match result {
        Ok(grid) => render_grid(grid, explicit_weeks),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_math_within_terminal_width() {
        for term_width in [40usize, 60, 80, 100, 110, 120] {
            let available = term_width.saturating_sub(5);
            let max_cols = (available / 2).max(1);
            let rendered_line_len = 4 + max_cols * 2;
            // The rendered line length MUST fit within the terminal width
            assert!(
                rendered_line_len <= term_width,
                "Rendered width {} exceeded terminal width {}",
                rendered_line_len,
                term_width
            );
        }
    }

    #[test]
    fn test_fit_weeks_to_terminal_explicit() {
        let mut weeks = Vec::new();
        for _ in 0..52 {
            weeks.push(WeekData {
                days: [None, None, None, None, None, None, None],
            });
        }
        let fitted = fit_weeks_to_terminal(weeks, Some(26));
        assert_eq!(fitted.len(), 26);
    }

    #[test]
    fn test_calculate_streaks_active_and_pending() {
        let today = Local::now().date_naive();
        let yest = today - Duration::days(1);
        let day_before = today - Duration::days(2);

        // Case 1: Active streak (contributed today + yesterday + day before)
        let week_active = WeekData {
            days: [
                Some(DayData {
                    date: day_before.format("%Y-%m-%d").to_string(),
                    count: 3,
                    level: 2,
                }),
                Some(DayData {
                    date: yest.format("%Y-%m-%d").to_string(),
                    count: 2,
                    level: 2,
                }),
                Some(DayData {
                    date: today.format("%Y-%m-%d").to_string(),
                    count: 5,
                    level: 3,
                }),
                None,
                None,
                None,
                None,
            ],
        };

        let stats_active = calculate_streaks(&[week_active]);
        assert_eq!(stats_active.current_streak, 3);
        assert_eq!(stats_active.today_count, 5);
        assert!(stats_active.is_active_today);
        let formatted_active = format_streak(&stats_active);
        assert!(formatted_active.contains("3 days (5 today)"));

        // Case 2: Pending streak (0 today, but contributed yesterday and day before)
        let week_pending = WeekData {
            days: [
                Some(DayData {
                    date: day_before.format("%Y-%m-%d").to_string(),
                    count: 3,
                    level: 2,
                }),
                Some(DayData {
                    date: yest.format("%Y-%m-%d").to_string(),
                    count: 2,
                    level: 2,
                }),
                Some(DayData {
                    date: today.format("%Y-%m-%d").to_string(),
                    count: 0,
                    level: 0,
                }),
                None,
                None,
                None,
                None,
            ],
        };

        let stats_pending = calculate_streaks(&[week_pending]);
        assert_eq!(stats_pending.current_streak, 2);
        assert_eq!(stats_pending.today_count, 0);
        assert!(!stats_pending.is_active_today);
        let formatted_pending = format_streak(&stats_pending);
        assert!(formatted_pending.contains("2 days (0 today)"));

        // Case 3: Broken streak (0 today and 0 yesterday)
        let week_broken = WeekData {
            days: [
                Some(DayData {
                    date: day_before.format("%Y-%m-%d").to_string(),
                    count: 3,
                    level: 2,
                }),
                Some(DayData {
                    date: yest.format("%Y-%m-%d").to_string(),
                    count: 0,
                    level: 0,
                }),
                Some(DayData {
                    date: today.format("%Y-%m-%d").to_string(),
                    count: 0,
                    level: 0,
                }),
                None,
                None,
                None,
                None,
            ],
        };

        let stats_broken = calculate_streaks(&[week_broken]);
        assert_eq!(stats_broken.current_streak, 0);
        assert_eq!(stats_broken.today_count, 0);
        assert!(!stats_broken.is_active_today);
    }

    #[test]
    fn test_month_header_omits_incomplete_month_labels() {
        // Week 0: October (only 1 week)
        // Week 1..5: November (4 weeks)
        let mut weeks = Vec::new();
        weeks.push(WeekData {
            days: [
                Some(DayData {
                    date: "2026-10-31".to_string(),
                    count: 1,
                    level: 1,
                }),
                None, None, None, None, None, None,
            ],
        });
        for _ in 1..=4 {
            weeks.push(WeekData {
                days: [
                    Some(DayData {
                        date: "2026-11-15".to_string(),
                        count: 1,
                        level: 1,
                    }),
                    None, None, None, None, None, None,
                ],
            });
        }

        let header = build_month_header(&weeks);
        // "Oct" must NOT appear as "Oc" or collide with "Nov"
        assert!(!header.contains("OcNov"), "Header contained collided 'OcNov'");
        assert!(!header.starts_with("Oc"), "Header started with incomplete 'Oc'");
        assert!(header.contains("Nov"), "Header must contain full 'Nov'");
    }
}
