use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use notify_rust::Notification;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    work_mins: u64,
    short_break_mins: u64,
    long_break_mins: u64,
    cycles: u64,
    bell: bool,
    notification: bool,
}

impl Config {
    // default pomodoro settings
    pub const DEFAULT: Self = Self {
        work_mins: 25,
        short_break_mins: 5,
        long_break_mins: 15,
        cycles: 4,
        bell: true,
        notification: true,
    };

    // desktime "golden ratio" work settings
    pub const DESKTIME: Self = Self {
        work_mins: 52,
        short_break_mins: 17,
        long_break_mins: 17,
        cycles: 4,
        bell: false,
        notification: true,
    };

    // ultradian rhythm deep work settings
    pub const FLOW: Self = Self {
        work_mins: 90,
        short_break_mins: 10,
        long_break_mins: 10,
        cycles: 4,
        bell: false,
        notification: false,
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

        fs::read_to_string(path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or(Self::DEFAULT)
    }

    pub fn save_default_if_missing() {
        if let Some(path) = Self::config_path() {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(&Self::DEFAULT) {
                    let _ = fs::write(path, json);
                }
            }
        }
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
        &_ => "",
    }
}

fn help_usage(cmd: &str) -> &str {
    match cmd {
        "start" => "[TASK] [FLAGS]",
        "break" => "[TASK] [FLAGS]",
        "help" => "[COMMAND]",
        &_ => "",
    }
}

fn help_flags() {
    println!("\t-v, --version            prints version information");
    println!(
        "\t--config                 prints config json file path---edit this to change default settings!"
    );
    println!("\t-a, --ago                subtracts specified minutes from timer");
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
fn value_flag(args: &Vec<String>, short_flag: &str, long_flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == short_flag || arg == long_flag)
        .and_then(|pos| args.get(pos + 1).cloned())
}

fn number_flag(args: &Vec<String>, short_flag: &str, long_flag: &str, default: u64) -> u64 {
    value_flag(args, short_flag, long_flag)
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(default)
}

fn trigger_alert(title: &str, body: &str, conf: &Config) {
    // 1. Terminal audio bell
    if conf.bell {
        print!("\x07");
        let _ = io::stdout().flush();
    }
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

fn run_timer(duration_secs: u64, initial_elapsed: u64, task: &str, tips: &[&str]) -> bool {
    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(_) => return false,
    };

    let mut stdout = io::stdout();
    let start_time = Instant::now();
    let mut total_paused = Duration::from_secs(0);
    let mut pause_start: Option<Instant> = None;
    let tip_length = tips.len();

    // Print initial blank lines so cursor movement won't overflow the screen top
    println!("\n\n\n\n");

    loop {
        // Handle Input
        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') => return false, // Cancelled
                    KeyCode::Char('s') => return true,  // Skipped/Completed
                    KeyCode::Char('p') | KeyCode::Char(' ') => {
                        if let Some(p_start) = pause_start {
                            total_paused += p_start.elapsed();
                            pause_start = None;
                        } else {
                            pause_start = Some(Instant::now());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Calculate time correctly with pausing
        let elapsed_active = match pause_start {
            Some(p_start) => (p_start - start_time) - total_paused,
            None => start_time.elapsed() - total_paused,
        };

        let total_elapsed = elapsed_active.as_secs() + initial_elapsed;

        if total_elapsed >= duration_secs {
            return true;
        }

        let remaining = duration_secs - total_elapsed;
        let mins = remaining / 60;
        let secs = remaining % 60;
        let current_tip = tips[(total_elapsed / 60) as usize % tip_length];
        let pause_status = if pause_start.is_some() {
            "  (PAUSED)"
        } else {
            ""
        };

        // Render line-by-line to prevent twitching
        write!(
            stdout,
            "{}{}{}Task: {}\n{}{}\t🏝️    {:02}:{:02}  🏝️{}\n{}{}{}\n{}{}[p]ause  [s]kip  [q]uit",
            cursor::MoveUp(3),
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            if task.is_empty() { "None" } else { task },
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            mins,
            secs,
            pause_status,
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
            current_tip,
            cursor::MoveToColumn(0),
            Clear(ClearType::UntilNewLine),
        )
        .unwrap();

        stdout.flush().unwrap();
    }
}

fn run_pomodoro_session(conf: &Config, task: Option<&str>, mut ago: u64) {
    let mut cycle = 1;

    loop {
        // Work Session
        let work_title = format!("Work Cycle #{}", cycle);
        // trigger_alert(
        //     "Pomodoro Started",
        //     &format!("Focus time! Cycle #{}", cycle),
        //     conf,
        // );
        let work_done = run_timer(
            conf.work_mins * 60,
            ago * 60,
            task.unwrap_or(work_title.as_str()),
            WORK_TIPS,
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

        // Break Session
        trigger_alert(
            "Time for a break!",
            &format!("Take a {} minute rest.", break_mins),
            conf,
        );
        let break_done = run_timer(break_mins * 60, 0, &label, BREAK_TIPS);

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

    if args.iter().any(|arg| arg == "--version" || arg == "-v") {
        println!("Version - {}", env!("CARGO_PKG_VERSION"))
    }

    if args.iter().any(|arg| arg == "--config") {
        println!(
            "Config Path - {}",
            Config::config_path()
                .unwrap_or_else(|| PathBuf::from("No config path available"))
                .display()
        );
    }

    let mut conf = if args.iter().any(|arg| arg == "-f" || arg == "--flow") {
        Config::FLOW
    } else if args.iter().any(|arg| arg == "-d" || arg == "--desktime") {
        Config::DESKTIME
    } else {
        Config::load()
    };
    Config::save_default_if_missing();

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
    let ago = number_flag(&args, "-a", "--ago", 0);

    match match_shortcut(args[1].as_str()) {
        "start" => run_pomodoro_session(&conf, get_piped_task().as_deref(), ago),
        "break" => {
            let task = get_piped_task().unwrap_or_else(|| {
                if args.len() > 2 && !args[2].starts_with('-') {
                    args[2].clone()
                } else {
                    String::from("No task provided")
                }
            });
            run_timer(conf.short_break_mins * 60, ago * 60, &task, BREAK_TIPS);
        }
        &_ => do_help(&args),
    }
}
