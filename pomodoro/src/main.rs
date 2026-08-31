use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, size},
};
use notify_rust::Notification;
use rand::{rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const CMD_NAME: &str = "palmtree";
const WORK_TIPS: &[&str] = &[
    "🎯 Focus on the task at hand. Avoid context switching.",
    "📵 Stay focused, no multitasking or checking your phone.",
];
const BREAK_TIPS: &[&str] = &[
    "Look out a window at a distant object for 20 seconds to rest your eyes.",
    "Grab a glass of water—hydrate your body and clear your mind.",
    "Stand up, stretch your shoulders, and take three deep, slow breaths.",
    "Step away from the screen; let your mind process in the background.",
    "Unclench your jaw, drop your shoulders, and relax your posture.",
];
const CONFIG_VERSION: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    version: u8,
    work_mins: u64,
    short_break_mins: u64,
    long_break_mins: u64,
    cycles: u64,
    bell: bool,
    notification: bool,
    #[serde(default)]
    wait: bool,
    #[serde(default)]
    bell_interval_secs: u64,
}

impl Config {
    // default pomodoro settings
    pub const DEFAULT: Self = Self {
        version: CONFIG_VERSION,
        work_mins: 25,
        short_break_mins: 5,
        long_break_mins: 15,
        cycles: 4,
        bell: true,
        notification: true,
        wait: false,
        bell_interval_secs: 10,
    };

    // desktime "golden ratio" work settings
    pub const DESKTIME: Self = Self {
        version: CONFIG_VERSION,
        work_mins: 52,
        short_break_mins: 17,
        long_break_mins: 17,
        cycles: 4,
        bell: false,
        notification: true,
        wait: false,
        bell_interval_secs: 10,
    };

    // ultradian rhythm deep work settings
    pub const FLOW: Self = Self {
        version: CONFIG_VERSION,
        work_mins: 90,
        short_break_mins: 10,
        long_break_mins: 10,
        cycles: 4,
        bell: false,
        notification: false,
        wait: false,
        bell_interval_secs: 10,
    };

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("palmtree").join("config.json"))
    }

    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::DEFAULT,
        };

        if !path.exists() {
            return Self::DEFAULT;
        }

        let contents = match fs::read_to_string(path.clone()) {
            Ok(c) => c,
            Err(_) => {
                println!(
                    "Error: Config file at {} could not be read.",
                    path.display()
                );
                return Self::DEFAULT;
            }
        };

        let conf: Config = match serde_json::from_str(&contents) {
            Ok(c) => c,
            Err(_) => {
                println!(
                    "Error: Config file could not be parsed. If modified, check for invalid syntax, else fix manually at {} or with `{} save`",
                    path.display(),
                    CMD_NAME
                );
                return Self::DEFAULT;
            }
        };
        if conf.version == CONFIG_VERSION {
            conf
        } else {
            println!(
                "Out of date config: version {} =/= {}\nUpdate manually at {} or use `{} save`",
                conf.version,
                CONFIG_VERSION,
                path.display(),
                CMD_NAME
            );
            Self::DEFAULT
        }
    }

    pub fn save(conf: &Config) -> bool {
        if let Some(path) = Self::config_path() {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    println!("Creating directories...");
                    let _ = fs::create_dir_all(parent);
                } else {
                    println!("Error: Failed to create all directories.");
                    return false;
                }
            }
            if let Ok(json) = serde_json::to_string_pretty(conf) {
                println!("Writing to file...");
                return fs::write(path, json).is_ok();
            } else {
                println!("Error: Failed to serialize config.");
            }
        } else {
            println!("Error: Can't save config file because path doesn't exist.");
        }
        false
    }
}

// super smart data structure to prevent program crash
// from leaving terminal in raw mode (and breaking it)
pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> Result<Self, io::Error> {
        if std::io::stdin().is_terminal() {
            let _ = enable_raw_mode();
            let _ = execute!(io::stdout(), cursor::Hide);
        }
        Ok(Self) // return self to make sure value not dropped until desired
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if std::io::stdin().is_terminal() {
            let _ = execute!(io::stdout(), cursor::Show);
            let _ = disable_raw_mode();
        }
    }
}

// ======================== HELP FUNCTIONALITY ========================
fn match_shortcut(cmd: &str) -> &str {
    match cmd {
        "s" => "start",
        "b" => "break",
        &_ => cmd,
    }
}

fn help_desc(cmd: &str) -> &str {
    match cmd {
        "start" => "starts a pomodoro session at the first work timer",
        "break" => "starts a standalone break timer with the short break duration",
        "save" => "saves specified config settings to the configuration file",
        &_ => "",
    }
}

fn help_usage(cmd: &str) -> &str {
    match cmd {
        "start" => "[TASK] [FLAGS]",
        "break" => "[TASK] [FLAGS]",
        "save" => "[FLAGS]",
        "help" => "[COMMAND]",
        &_ => "",
    }
}

fn help_flags() {
    println!("\t-v, --version            prints version information");
    println!("\t--config                 prints config json file path---edit more settings here!");
    println!(
        "\t-p, --print              prints the current config settings the program is running with after modifications"
    );
    println!(
        "\t-a, --ago                subtracts specified minutes from the first timer (not in config)"
    );
    println!(
        "\t--wait                   waits (and plays bell if on) until you unpause to end the timer"
    );
    println!("\t-w, --work               sets work timer length in minutes");
    println!("\t-s, --short              sets short break length in minutes");
    println!("\t-l, --long               sets long break length in minutes");
    println!("\t-c, --cycles             sets work cycles before long break");
    println!("\t-b, --bell-off           disables system bell sound on timer completion");
    println!("\t-n, --no-notification    disables desktop notification on timer completion");
    println!("\t-d, --desktime           uses DeskTime base config (52/17 productivity ratio)");
    println!("\t-f, --flow               uses flow base config (ultradian rhythm, deep work)");
}

fn do_help(args: &Vec<String>) {
    if args.len() > 2 {
        // Command-specific help
        let cmd = match_shortcut(args[2].as_str());
        let desc = help_desc(cmd);

        if desc.is_empty() {
            println!(
                "Unknown command '{}'. Run '{} help' for available commands.",
                args[2], CMD_NAME
            );
            return;
        }

        println!("\nNAME:");
        println!("\t{}-{} - {}\n", CMD_NAME, cmd, desc);

        println!("USAGE:");
        println!("\t{} {} {}\n", CMD_NAME, cmd, help_usage(cmd));

        println!("FLAGS:");
        help_flags();
        println!();
    } else {
        // General top-level help
        println!("Usage: {} [COMMAND] [FLAGS]\n", CMD_NAME);

        println!("Commands:");
        println!("\tstart, s                {}", help_desc("start"));
        println!("\tbreak, b                {}", help_desc("break"));

        println!("Flags:");
        help_flags();

        println!(
            "\nUse \"{} help [COMMAND]\" for more information on a specific command.",
            CMD_NAME
        );
    }
}

// ======================== TIMER FUNCTIONALITY ========================
fn value_flag(args: &[String], short_flag: &str, long_flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == short_flag || arg == long_flag)
        .and_then(|pos| args.get(pos + 1).cloned())
}

fn number_flag(args: &[String], short_flag: &str, long_flag: &str, default: u64) -> u64 {
    value_flag(args, short_flag, long_flag)
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(default)
}

// fn trigger_bell(conf: &Config) {
//     // 1. Terminal audio bell
//     if conf.bell {
//         print!("\x07");
//         let _ = io::stdout().flush();
//         // exit(0)
//     }
// }

fn trigger_alert(title: &str, body: &str, conf: &Config) {
    // 2. Desktop Notification
    if conf.notification {
        let _ = Notification::new()
            .summary(title)
            .body(body)
            .appname(CMD_NAME)
            .show();
    }
}

fn get_piped_task() -> Option<String> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        let mut buffer = String::new();
        if stdin.lock().read_to_string(&mut buffer).is_ok() {
            let line = buffer.lines().next().unwrap_or("").trim().to_string();
            if !line.is_empty() {
                return Some(line);
            }
        }
    }
    None
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count > max_len {
        if max_len <= 3 {
            s.chars().take(max_len).collect()
        } else {
            let truncated: String = s.chars().take(max_len - 3).collect();
            format!("{}...", truncated)
        }
    } else {
        s.to_string()
    }
}

fn run_timer(
    duration_secs: u64,
    initial_elapsed: u64,
    task: &str,
    tips: &[&str],
    conf: &Config,
) -> bool {
    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(_) => return false,
    };

    // drain any leftover events because they'll make me angry!! >:(
    while event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }

    let mut stdout = io::stdout();
    let start_time = Instant::now();
    let mut total_paused = Duration::from_secs(0);
    let mut pause_start: Option<Instant> = None;
    let tip_length = tips.len();
    let mut to_wait = conf.wait;
    let mut waiting = false;
    let mut bell_intervals = 0;

    // Print initial blank lines so cursor movement won't overflow the screen top
    println!("\n\n\n\n");

    loop {
        // Handle Input
        if event::poll(Duration::from_millis(100)).unwrap_or(false)
            && let Ok(Event::Key(key)) = event::read()
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => {
                    println!();
                    return false; // Cancelled
                }
                KeyCode::Char('s') => {
                    println!();
                    return true; // Skipped/Completed
                }
                KeyCode::Char('p') | KeyCode::Char(' ') => {
                    if let Some(p_start) = pause_start {
                        total_paused += p_start.elapsed();
                        pause_start = None;
                        waiting = false;
                    } else {
                        pause_start = Some(Instant::now());
                    }
                }
                KeyCode::Char('w') => {
                    to_wait = !to_wait;
                    if !to_wait && waiting {
                        waiting = false;
                        pause_start = None;
                    }
                }
                _ => {}
            }
        }

        // Calculate time correctly with pausing
        let elapsed_active = match pause_start {
            Some(p_start) => (p_start - start_time) - total_paused,
            None => start_time.elapsed() - total_paused,
        };

        let total_elapsed = elapsed_active.as_secs() + initial_elapsed;

        let mut add_bell = false;
        if total_elapsed >= duration_secs {
            if let Some(start) = pause_start {
                let intervals = start.elapsed().as_secs() / conf.bell_interval_secs;
                if intervals > bell_intervals {
                    add_bell = true;
                    bell_intervals = intervals;
                }
            } else {
                if !to_wait {
                    // Play end-of-timer bell cleanly
                    if conf.bell {
                        print!("\x07");
                        let _ = io::stdout().flush();
                        // Give the terminal time to process the sound before the buffer wipes
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    return true;
                }
                add_bell = true;
                pause_start = Some(Instant::now());
                waiting = true;
            }
        }

        let remaining = duration_secs - total_elapsed;
        let mins = remaining / 60;
        let secs = remaining % 60;
        let raw_tip = tips[(total_elapsed / 60) as usize % tip_length];
        let mut pause_status = if let Some(start) = pause_start {
            let elapsed = start.elapsed().as_secs();
            if waiting {
                format!("  (WAITING {:02}:{:02})", elapsed / 60, elapsed % 60)
            } else {
                format!("  (PAUSED {:02}:{:02})", elapsed / 60, elapsed % 60)
            }
        } else {
            String::new()
        };
        if add_bell {
            pause_status.push_str("\x07");
        }

        let (term_cols, _) = size().unwrap_or((80, 24));
        let max_width = term_cols as usize;

        // Truncate task and tip if they exceed window bounds
        let task_prefix = "Task: ";
        let raw_task = if task.is_empty() { "None" } else { task };
        let display_task = truncate_str(raw_task, max_width.saturating_sub(task_prefix.len()));
        let display_tip = truncate_str(raw_tip, max_width);

        // Render line-by-line to prevent twitching
        write!(
            stdout,
            "{}{}{}Task: {}\n{}{}\t🏝️    {:02}:{:02}  🏝️{}\n{}{}{}\n{}{}[p]ause  [w]ait ({})  [s]kip  [q]uit",
            cursor::MoveUp(3),
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            display_task,
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            mins,
            secs,
            pause_status,
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            display_tip,
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            if to_wait {
                "◼"
            } else {
                "◻"
            }
        )
        .expect("Error writing to stdout.");

        stdout.flush().expect("Error flushing stdout.");

        // Trigger interval bell cleanly AFTER the UI frame is fully drawn
        if add_bell && waiting && conf.bell {
            print!("\x07");
            let _ = io::stdout().flush();
        }
    }
}

fn run_pomodoro_session(conf: &Config, task: Option<&str>, mut ago: u64) {
    let mut cycle = 1;
    let mut rng = rng();
    let mut break_tips = BREAK_TIPS.to_owned();

    loop {
        // Work Session
        let work_title = format!("Work Cycle #{}", cycle);
        trigger_alert(
            "Pomodoro Started",
            &format!("Focus time! Cycle #{}", cycle),
            conf,
        );
        let work_done = run_timer(
            conf.work_mins * 60,
            ago * 60,
            task.unwrap_or(work_title.as_str()),
            WORK_TIPS,
            conf,
        );
        ago = 0;

        if !work_done {
            println!("\nSession cancelled.");
            break;
        }

        // Check if long break or short break
        let is_long_break = cycle % conf.cycles == 0;
        let break_mins = if is_long_break {
            conf.long_break_mins
        } else {
            conf.short_break_mins
        };
        let break_title = if is_long_break {
            "Long Break"
        } else {
            "Short Break"
        };
        let label = match task {
            Some(t) => format!("{} ({})", break_title, t),
            None => break_title.to_string(),
        };

        // Shuffle break tips
        break_tips.shuffle(&mut rng);

        // Break Session
        trigger_alert(
            "Time for a break!",
            &format!("Take a {} minute rest.", break_mins),
            conf,
        );
        let break_done = run_timer(break_mins * 60, 0, &label, &break_tips, conf);

        if !break_done {
            println!("\nSession cancelled during break.");
            break;
        }

        cycle += 1;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Welcome to Max's pomodoro timer cli!");
        println!("Version - {}\n", env!("CARGO_PKG_VERSION"));
        do_help(&args);
        return;
    }

    let mut help = true;
    if args.iter().any(|arg| arg == "--version" || arg == "-v") {
        println!("Version - {}", env!("CARGO_PKG_VERSION"));
        help = false;
    }

    if args.iter().any(|arg| arg == "--config") {
        println!(
            "Config Path - {}",
            Config::config_path()
                .unwrap_or_else(|| PathBuf::from("No config path available"))
                .display()
        );
        println!("Expected Config Version - {}", CONFIG_VERSION);
        help = false;
    }

    let mut conf = if args.iter().any(|arg| arg == "-f" || arg == "--flow") {
        Config::FLOW
    } else if args.iter().any(|arg| arg == "-d" || arg == "--desktime") {
        Config::DESKTIME
    } else {
        Config::load()
    };

    // use flags like -w, -s, -l, -c to modify AFTER base config
    conf.work_mins = number_flag(&args, "-w", "--work", conf.work_mins);
    conf.short_break_mins = number_flag(&args, "-s", "--short", conf.short_break_mins);
    conf.long_break_mins = number_flag(&args, "-l", "--long", conf.long_break_mins);
    conf.cycles = number_flag(&args, "-c", "--cycles", conf.cycles);
    conf.bell = !args.iter().any(|arg| arg == "-b" || arg == "--bell-off") && conf.bell;
    conf.notification = !args
        .iter()
        .any(|arg| arg == "-n" || arg == "--no-notification")
        && conf.notification;
    conf.wait = args.iter().any(|arg| arg == "--wait") || conf.wait;
    let ago = number_flag(&args, "-a", "--ago", 0);

    if args.iter().any(|arg| arg == "--print" || arg == "-p") {
        println!(
            "Running with Config:\n\tversion = {}\n\twork_mins = {}\n\tshort_break_mins = {}\n\tlong_break_mins = {}\n\tcycles = {}\n\tbell = {}\n\tnotification = {}\n\twait = {}\n\tbell_interval_secs = {}",
            conf.version,
            conf.work_mins,
            conf.short_break_mins,
            conf.long_break_mins,
            conf.cycles,
            conf.bell,
            conf.notification,
            conf.wait,
            conf.bell_interval_secs
        )
    }

    match match_shortcut(args[1].as_str()) {
        "start" => {
            let piped_task = get_piped_task();
            let task = if let Some(t) = &piped_task {
                Some(t.as_str())
            } else if args.len() > 2 && !args[2].starts_with('-') {
                Some(args[2].as_str())
            } else {
                None
            };
            run_pomodoro_session(&conf, task, ago)
        }
        "break" => {
            let piped_task = get_piped_task();
            let task = if let Some(t) = &piped_task {
                t.as_str()
            } else if args.len() > 2 && !args[2].starts_with('-') {
                args[2].as_str()
            } else {
                "No task provided"
            };

            run_timer(
                conf.short_break_mins * 60,
                ago * 60,
                task,
                BREAK_TIPS,
                &conf,
            );
        }
        "save" => {
            println!("Saving Config...");
            if Config::save(&conf) {
                println!("Successfully saved!")
            } else {
                println!("Save failed.");
            };
        }
        "help" => do_help(&args),
        &_ => {
            if help {
                do_help(&args)
            }
        }
    }
}
