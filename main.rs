use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;


#[cfg(not(target_os = "linux"))]
fn check_linux() {
    eprintln!(
        "\n{} {}",
        "✗".red().bold(),
        "ClearLinux only runs on Linux!".red().bold()
    );
    eprintln!(
        "  {} Detected OS: {}",
        "→".yellow(),
        std::env::consts::OS
    );
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn check_linux() {}

fn print_banner() {
    println!();
    println!("{}", "╔══════════════════════════════════════════════╗".cyan().bold());
    println!("{}", "║                                              ║".cyan().bold());
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        " ██████╗██╗     ███████╗ █████╗ ██████╗     ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        "██╔════╝██║     ██╔════╝██╔══██╗██╔══██╗    ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        "██║     ██║     █████╗  ███████║██████╔╝    ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        "██║     ██║     ██╔══╝  ██╔══██║██╔══██╗    ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        "╚██████╗███████╗███████╗██║  ██║██║  ██║    ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!(
        "{}  {}  {}",
        "║".cyan().bold(),
        " ╚═════╝╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝   ".bright_cyan().bold(),
        "║".cyan().bold()
    );
    println!("{}", "║                                              ║".cyan().bold());
    println!(
        "{}          {}          {}",
        "║".cyan().bold(),
        "Linux System Cleaner v1.0".white().bold(),
        "║".cyan().bold()
    );
    println!("{}", "╚══════════════════════════════════════════════╝".cyan().bold());
    println!();
}


fn detect_distro() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line
                    .trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
                return name;
            }
        }
    }
    "Unknown Linux".to_string()
}

fn print_system_info() {
    let distro = detect_distro();

    // Kernel version
    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Uptime
    let uptime = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|v| v.parse::<f64>().ok()).flatten())
        .map(|secs| {
            let h = (secs as u64) / 3600;
            let m = ((secs as u64) % 3600) / 60;
            format!("{}h {}m", h, m)
        })
        .unwrap_or_else(|| "unknown".to_string());

    
    let disk = Command::new("df")
        .args(["-h", "--output=used,avail,pcent", "/"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.lines().nth(1).map(|l| l.split_whitespace().collect::<Vec<_>>().join("  /  "))
        })
        .unwrap_or_else(|| "unavailable".to_string());

    println!("{}", "┌─ System Info ───────────────────────────────┐".cyan());
    println!("{}  {}  {}", "│".cyan(), format!("🐧  Distro : {}", distro).white(), "".cyan());
    println!("{}  {}  {}", "│".cyan(), format!("🔧  Kernel : {}", kernel).white(), "".cyan());
    println!("{}  {}  {}", "│".cyan(), format!("⏱   Uptime : {}", uptime).white(), "".cyan());
    println!("{}  {}  {}", "│".cyan(), format!("💾  Disk   : {} (used / free / %)", disk).white(), "".cyan());
    println!("{}", "└─────────────────────────────────────────────┘".cyan());
    println!();
}


fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = fs::metadata(&p) {
                total += meta.len();
            }
        }
    }
    total
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_files(&p, files);
            } else {
                files.push(p);
            }
        }
    }
}


struct ScanResult {
    name: String,
    description: String,
    size: u64,
    paths: Vec<PathBuf>,
}

fn scan_targets() -> Vec<ScanResult> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

    let scan_dirs: Vec<(&str, &str, Vec<String>)> = vec![
        (
            "Package Cache (APT)",
            "Cached .deb packages from apt",
            vec!["/var/cache/apt/archives".to_string()],
        ),
        (
            "Package Cache (pacman)",
            "Cached packages from pacman",
            vec!["/var/cache/pacman/pkg".to_string()],
        ),
        (
            "Package Cache (dnf/yum)",
            "Cached RPM packages",
            vec![
                "/var/cache/dnf".to_string(),
                "/var/cache/yum".to_string(),
            ],
        ),
        (
            "Thumbnail Cache",
            "Auto-generated image thumbnails",
            vec![format!("{}/.cache/thumbnails", home)],
        ),
        (
            "Browser Cache (Chromium)",
            "Chromium browser cached data",
            vec![format!("{}/.cache/chromium", home)],
        ),
        (
            "Browser Cache (Firefox)",
            "Firefox browser cached data",
            vec![format!("{}/.cache/mozilla/firefox", home)],
        ),
        (
            "Trash",
            "Files in your trash bin",
            vec![
                format!("{}/.local/share/Trash/files", home),
                "/root/.local/share/Trash/files".to_string(),
            ],
        ),
        (
            "Old Logs (/var/log)",
            "Rotated & compressed log files (*.gz, *.old, *.1)",
            vec!["/var/log".to_string()],
        ),
        (
            "Temp Files (/tmp)",
            "Temporary system files",
            vec!["/tmp".to_string()],
        ),
        (
            "User Cache (~/.cache)",
            "General application cache",
            vec![format!("{}/.cache", home)],
        ),
        (
            "Snap Cache",
            "Snap package cache",
            vec![format!("{}/.cache/snapd", home)],
        ),
        (
            "Flatpak Cache",
            "Flatpak app cache",
            vec![format!("{}/.cache/flatpak", home)],
        ),
    ];

    let mut results: Vec<ScanResult> = Vec::new();

    for (name, desc, dirs) in &scan_dirs {
        pb.set_message(format!("Scanning {}...", name));

        let mut files: Vec<PathBuf> = Vec::new();
        let mut total: u64 = 0;

        for dir in dirs {
            let p = Path::new(dir);
            // For old logs: only .gz/.old/.[0-9] files
            if *name == "Old Logs (/var/log)" {
                if p.exists() {
                    if let Ok(entries) = fs::read_dir(p) {
                        for entry in entries.flatten() {
                            let ep = entry.path();
                            let fname = ep.file_name().unwrap_or_default().to_string_lossy().to_string();
                            if fname.ends_with(".gz")
                                || fname.ends_with(".old")
                                || fname.ends_with(".1")
                                || fname.ends_with(".2")
                                || fname.ends_with(".3")
                            {
                                if let Ok(m) = fs::metadata(&ep) {
                                    total += m.len();
                                    files.push(ep);
                                }
                            }
                        }
                    }
                }
            } else {
                total += dir_size(p);
                collect_files(p, &mut files);
            }
        }

        if total > 0 {
            results.push(ScanResult {
                name: name.to_string(),
                description: desc.to_string(),
                size: total,
                paths: dirs.iter().map(PathBuf::from).collect(),
            });
        }
    }

    pb.finish_and_clear();
    results
}


fn print_scan_table(results: &[ScanResult]) {
    println!("{}", "┌─ Scan Results ──────────────────────────────────────────────────┐".cyan());
    println!(
        "{}  {:<35} {:<18} {}",
        "│".cyan(),
        "Category".white().bold(),
        "Size".white().bold(),
        "Description".white().bold()
    );
    println!("{}", "├─────────────────────────────────────────────────────────────────┤".cyan());

    let total: u64 = results.iter().map(|r| r.size).sum();

    for r in results {
        let size_str = format_size(r.size, BINARY);
        let color_size = if r.size > 500_000_000 {
            size_str.red().bold()
        } else if r.size > 100_000_000 {
            size_str.yellow().bold()
        } else {
            size_str.green()
        };

        println!(
            "{}  {:<35} {:<28} {}",
            "│".cyan(),
            r.name.white(),
            color_size,
            r.description.bright_black()
        );
    }

    println!("{}", "├─────────────────────────────────────────────────────────────────┤".cyan());
    println!(
        "{}  {:<35} {}",
        "│".cyan(),
        "TOTAL".white().bold(),
        format_size(total, BINARY).green().bold()
    );
    println!("{}", "└─────────────────────────────────────────────────────────────────┘".cyan());
    println!();
}


fn clean_path(path: &Path, log_name: &str) -> (u64, u32) {
    let mut freed: u64 = 0;
    let mut count: u32 = 0;

    if !path.exists() {
        return (freed, count);
    }

    if path.is_file() {
        if let Ok(m) = fs::metadata(path) {
            freed += m.len();
        }
        let _ = fs::remove_file(path);
        count += 1;
        return (freed, count);
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let (f, c) = clean_path(&p, log_name);
                freed += f;
                count += c;
                let _ = fs::remove_dir(&p);
            } else {
                if let Ok(m) = fs::metadata(&p) {
                    freed += m.len();
                }
                let _ = fs::remove_file(&p);
                count += 1;
            }
        }
    }

    (freed, count)
}

fn run_clean(selected: &[ScanResult]) {
    println!();
    println!("{}", "─── Cleaning ─────────────────────────────────────────────────────".cyan());

    let total_steps = selected.len() as u64;
    let pb = ProgressBar::new(total_steps);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}",
            )
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏ "),
    );

    let mut total_freed: u64 = 0;
    let mut total_files: u32 = 0;

    for result in selected {
        pb.set_message(format!("Cleaning {}…", result.name));

        let mut freed: u64 = 0;
        let mut count: u32 = 0;

        // Special handling for old logs
        if result.name == "Old Logs (/var/log)" {
            if let Ok(entries) = fs::read_dir("/var/log") {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if fname.ends_with(".gz")
                        || fname.ends_with(".old")
                        || fname.ends_with(".1")
                        || fname.ends_with(".2")
                        || fname.ends_with(".3")
                    {
                        if let Ok(m) = fs::metadata(&p) {
                            freed += m.len();
                        }
                        let _ = fs::remove_file(&p);
                        count += 1;
                    }
                }
            }
        } else {
            for path in &result.paths {
                let (f, c) = clean_path(path, &result.name);
                freed += f;
                count += c;
            }
        }

        total_freed += freed;
        total_files += count;
        pb.inc(1);
        std::thread::sleep(Duration::from_millis(120)); // visual polish
    }

    pb.finish_and_clear();

    println!();
    println!("{}", "┌─ Cleaning Complete ─────────────────────────────────────────────┐".green().bold());
    println!(
        "{}  {}  {}",
        "│".green(),
        format!("✔  Files removed : {}", total_files).white().bold(),
        "".green()
    );
    println!(
        "{}  {}  {}",
        "│".green(),
        format!("✔  Space freed   : {}", format_size(total_freed, BINARY)).bright_green().bold(),
        "".green()
    );
    println!("{}", "└─────────────────────────────────────────────────────────────────┘".green().bold());
}


fn main() {
    // 1. OS gate
    check_linux();

    // Ctrl-C handler
    ctrlc::set_handler(|| {
        println!("\n\n{} Aborted by user.", "✗".red().bold());
        std::process::exit(0);
    })
    .ok();

    // 2. Banner + system info
    print_banner();
    print_system_info();

    // 3. Scan
    println!("{} Scanning your system for junk…", "→".cyan().bold());
    println!();
    let results = scan_targets();

    if results.is_empty() {
        println!(
            "\n{} {}",
            "✓".green().bold(),
            "Your system is already clean! Nothing to remove.".green().bold()
        );
        return;
    }

    // 4. Show table
    print_scan_table(&results);

    // 5. Multi-select
    let labels: Vec<String> = results
        .iter()
        .map(|r| format!("{:<35} ({})", r.name, format_size(r.size, BINARY)))
        .collect();

    let defaults = vec![true; labels.len()];

    println!(
        "{} {}",
        "→".cyan().bold(),
        "Select categories to clean (Space to toggle, Enter to confirm):".white().bold()
    );
    println!();

    let chosen_indices = MultiSelect::with_theme(&ColorfulTheme::default())
        .items(&labels)
        .defaults(&defaults)
        .interact();

    let chosen_indices = match chosen_indices {
        Ok(v) => v,
        Err(_) => {
            println!("\n{} Cancelled.", "✗".yellow());
            return;
        }
    };

    if chosen_indices.is_empty() {
        println!("\n{} Nothing selected. Exiting.", "✗".yellow().bold());
        return;
    }

    let selected: Vec<&ScanResult> = chosen_indices.iter().map(|&i| &results[i]).collect();
    let total_size: u64 = selected.iter().map(|r| r.size).sum();

    println!();
    println!(
        "  {} You selected {} categories ({} will be freed).",
        "→".cyan(),
        chosen_indices.len().to_string().yellow().bold(),
        format_size(total_size, BINARY).yellow().bold()
    );
    println!();

    // 6. Confirm
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with cleaning?")
        .default(false)
        .interact();

    match confirm {
        Ok(true) => {
            // Flatten to owned vec for clean
            let owned: Vec<&ScanResult> = selected;
            let owned_results: Vec<ScanResult> = owned.into_iter().map(|r| ScanResult {
                name: r.name.clone(),
                description: r.description.clone(),
                size: r.size,
                paths: r.paths.clone(),
            }).collect();
            run_clean(&owned_results);
        }
        _ => {
            println!("\n{} Cleaning cancelled. Nothing was removed.", "✗".yellow().bold());
        }
    }

    println!();
    println!(
        "  {} {}",
        "★".bright_cyan(),
        "Thanks for using ClearLinux!".white().bold()
    );
    println!();
}
