//! ClaudeButton — repurpose weird HID devices into Claude control.
//!
//! Loads data-driven profiles (profiles/*.yaml|json), grabs each device's input
//! nodes, translates their events (gesture/keymap) into keystrokes, and injects
//! them via a shared uinput keyboard into the focused window. One reader thread
//! per device keeps it robust to the cheap gadgets' constant re-enumeration.
//!
//! Usage:
//!   claudebutton [PROFILES_DIR]     run the daemon (default dir: ./profiles)
//!   claudebutton capture [NAME]     scope a device's events to build a profile

mod gesture;
mod inject;
mod keys;
mod profile;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use evdev::{Device, EventType, InputEvent};

use inject::Injector;
use profile::{Profile, Translator};

/// println! + immediate flush. Rust's Stdout block-buffers to a pipe (journald),
/// so a plain println! would delay a daemon's logs; flush keeps them live.
macro_rules! logln {
    ($($a:tt)*) => {{
        println!($($a)*);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }};
}

enum Msg {
    Event { profile: String, ev: InputEvent },
    Gone { path: PathBuf },
}

/// (evdev name, /dev/input/eventN) for every input device, parsed from /proc.
fn list_nodes() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let text = match std::fs::read_to_string("/proc/bus/input/devices") {
        Ok(t) => t,
        Err(_) => return out,
    };
    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut handler = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("N: Name=") {
                name = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
                for tok in rest.split_whitespace() {
                    if tok.starts_with("event") {
                        handler = tok.to_string();
                    }
                }
            }
        }
        if !name.is_empty() && !handler.is_empty() {
            out.push((name, PathBuf::from(format!("/dev/input/{handler}"))));
        }
    }
    out
}

fn owner<'a>(profiles: &'a [Profile], node_name: &str) -> Option<&'a Profile> {
    profiles.iter().find(|p| p.match_spec.matches(node_name))
}

/// Grab a device and stream its events until it disappears.
fn reader(path: PathBuf, mut dev: Device, profile: String, tx: mpsc::Sender<Msg>) {
    let _ = dev.grab();
    'outer: while let Ok(events) = dev.fetch_events() {
        for ev in events {
            if tx
                .send(Msg::Event {
                    profile: profile.clone(),
                    ev,
                })
                .is_err()
            {
                break 'outer;
            }
        }
    }
    let _ = dev.ungrab();
    let _ = tx.send(Msg::Gone { path });
}

/// Fire-and-forget BlueZ (re)connect on a specific adapter, via bluetoothctl.
/// (v1: shell-out. Planned: the `bluer` crate for native connect + disconnect
/// signals, dropping this dependency and the /proc poll.)
fn nudge_reconnect(mac: &str, adapter: Option<&str>) {
    let mut script = String::new();
    if let Some(a) = adapter {
        script.push_str(&format!("select {a}\n"));
    }
    script.push_str(&format!("connect {mac}\n"));
    if let Ok(mut child) = Command::new("bluetoothctl")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut sin) = child.stdin.take() {
            let _ = sin.write_all(script.as_bytes());
        }
        // Reap it so short-lived bluetoothctl processes don't pile up as zombies.
        thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("capture") {
        return capture(args.get(2).map(String::as_str).unwrap_or(""));
    }
    run_daemon(args.get(1).cloned().unwrap_or_else(|| "profiles".into()))
}

fn run_daemon(profiles_dir: String) -> Result<()> {
    let profiles = profile::load_dir(Path::new(&profiles_dir))?;
    logln!(
        "ClaudeButton: loaded {} profile(s): {}",
        profiles.len(),
        profiles
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut injector = Injector::new()?;
    let mut translators: HashMap<String, Vec<Translator>> = HashMap::new();
    for p in &profiles {
        translators.insert(p.name.clone(), p.build_translators()?);
    }

    let (tx, rx) = mpsc::channel::<Msg>();
    let mut active: HashMap<PathBuf, String> = HashMap::new(); // path -> profile name
    let mut last_nudge: HashMap<String, Instant> = HashMap::new();
    let mut last_scan = Instant::now() - Duration::from_secs(10);

    loop {
        if last_scan.elapsed() >= Duration::from_millis(500) {
            for (name, path) in list_nodes() {
                if active.contains_key(&path) {
                    continue;
                }
                if let Some(p) = owner(&profiles, &name) {
                    match Device::open(&path) {
                        Ok(dev) => {
                            logln!("[+] {}: grabbed {} ({})", p.name, path.display(), name);
                            active.insert(path.clone(), p.name.clone());
                            let (txc, pn, pc) = (tx.clone(), p.name.clone(), path.clone());
                            thread::spawn(move || reader(pc, dev, pn, txc));
                        }
                        Err(e) => eprintln!("[!] {}: open {} failed: {e}", p.name, path.display()),
                    }
                }
            }
            for p in &profiles {
                if let Some(rc) = &p.reconnect {
                    let present = active.values().any(|n| n == &p.name);
                    let due = last_nudge
                        .get(&p.name)
                        .map(|t| t.elapsed() >= Duration::from_secs(8))
                        .unwrap_or(true);
                    if !present && due {
                        logln!("[~] {}: nudging reconnect {}", p.name, rc.mac);
                        nudge_reconnect(&rc.mac, rc.adapter.as_deref());
                        last_nudge.insert(p.name.clone(), Instant::now());
                    }
                }
            }
            last_scan = Instant::now();
        }

        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Msg::Event { profile, ev }) => {
                if let Some(ts) = translators.get_mut(&profile) {
                    for t in ts.iter_mut() {
                        if let Some(code) = t.handle(&ev) {
                            logln!("  {profile} -> {}", keys::name(code));
                            let _ = injector.tap(code);
                            break;
                        }
                    }
                }
            }
            Ok(Msg::Gone { path }) => {
                active.remove(&path);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

/// Scope a device's raw events (and gesture classification) so you can write a
/// profile for it. Grabs every input node whose name contains `substr`.
fn capture(substr: &str) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Msg>();
    let mut grabbed = 0;
    for (name, path) in list_nodes() {
        if name.contains(substr) {
            match Device::open(&path) {
                Ok(dev) => {
                    logln!("listening: {} ({})", path.display(), name);
                    let (txc, pn, pc) = (tx.clone(), name.clone(), path.clone());
                    thread::spawn(move || reader(pc, dev, pn, txc));
                    grabbed += 1;
                }
                Err(e) => eprintln!("open {} failed: {e}", path.display()),
            }
        }
    }
    if grabbed == 0 {
        bail!("no input nodes matched {substr:?} — device connected? (grab needs root)");
    }
    logln!("\nOperate the device (one action at a time). Ctrl-C to stop.\n");
    let mut engine = gesture::GestureEngine::new(400.0);
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Event { profile, ev } => {
                match ev.event_type() {
                    EventType::KEY => {
                        logln!(
                            "  [{profile}] KEY {} = {}",
                            keys::name(ev.code()),
                            ev.value()
                        )
                    }
                    EventType::ABSOLUTE => {
                        logln!(
                            "  [{profile}] ABS {} = {}",
                            keys::abs_name(ev.code()),
                            ev.value()
                        )
                    }
                    _ => {}
                }
                if let Some(g) = engine.feed(&ev) {
                    logln!("  => GESTURE {}", gesture::name(g));
                }
            }
            Msg::Gone { path } => logln!("  ({} vanished)", path.display()),
        }
    }
    Ok(())
}
