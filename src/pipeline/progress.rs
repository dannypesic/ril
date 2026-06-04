use std::io::Write;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

pub enum Msg {
    Total(usize),
    Batch(usize),
}

pub fn run(rx: Receiver<Msg>) {
    let mut total: Option<usize> = None;
    let mut current: usize = 0;
    let start = Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_millis(120)) {
            Ok(Msg::Total(n)) => { total = Some(n); }
            Ok(Msg::Batch(n)) => { current = n; }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        draw(current, total, start.elapsed().as_secs_f64());
    }

    finish(current, total, start.elapsed().as_secs_f64());
}

const BAR_WIDTH: usize = 22;

fn draw(current: usize, total: Option<usize>, elapsed: f64) {
    let err = std::io::stderr();
    let mut err = err.lock();

    let line = if let Some(total) = total.filter(|&t| t > 0) {
        let pct = (current as f64 / total as f64).min(1.0);
        let filled = ((pct * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
        let empty = BAR_WIDTH - filled;
        let eta = eta_str(current, total, elapsed);
        format!(
            "\r\x1b[2K  \x1b[1;36mril\x1b[0m  \
             \x1b[2m▕\x1b[0m\x1b[32m{filled}\x1b[0m\x1b[2m{empty}▏\x1b[0m  \
             \x1b[1m{pct:>3.0}%\x1b[0m  \
             \x1b[2m{current}/{total}\x1b[0m  \
             \x1b[33m~{eta}\x1b[0m",
            filled = "█".repeat(filled),
            empty = "░".repeat(empty),
            pct = pct * 100.0,
        )
    } else {
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let tick = (elapsed * 8.0) as usize % spinner.len();
        format!(
            "\r\x1b[2K  \x1b[1;36mril\x1b[0m  \x1b[36m{}\x1b[0m  \
             \x1b[2m{} batches  {}\x1b[0m",
            spinner[tick],
            current,
            fmt_duration(elapsed),
        )
    };

    let _ = write!(err, "{line}");
    let _ = err.flush();
}

fn finish(current: usize, total: Option<usize>, elapsed: f64) {
    let count = total.unwrap_or(current);
    let err = std::io::stderr();
    let mut err = err.lock();
    let _ = writeln!(
        err,
        "\r\x1b[2K  \x1b[1;36mril\x1b[0m  \x1b[32m✓\x1b[0m  \
         \x1b[2m{count} batches  done in {}\x1b[0m",
        fmt_duration(elapsed),
    );
}

fn eta_str(current: usize, total: usize, elapsed: f64) -> String {
    if current == 0 || elapsed < 0.5 {
        return "--".to_string();
    }
    let remaining = (total - current) as f64 * elapsed / current as f64;
    fmt_duration(remaining)
}

fn fmt_duration(secs: f64) -> String {
    let secs = secs.max(0.0);
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else {
        let m = (secs / 60.0) as u64;
        let s = secs as u64 % 60;
        format!("{}m{}s", m, s)
    }
}
