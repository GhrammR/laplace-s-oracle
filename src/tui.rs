//! The Panopticon: Real-time Terminal Visualization for Laplace Oracle.
//!
//! Enforces:
//! 1. Zero-allocation ring buffers for history.
//! 2. Asynchronous telemetry reading with cryptographic verification.
//! 3. Panic-safe terminal state restoration.

#![deny(clippy::all)]
#![allow(clippy::manual_is_multiple_of)]

use std::{
    error::Error,
    io::{self, Read, Write},
    panic,
    thread,
    time::{Duration, SystemTime},
    fs::OpenOptions,
};

use base64::Engine;
use crossbeam_channel::bounded;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame, Terminal,
};

use laplace_oracle::ipc::{MiracleCommand, MiracleType};
use laplace_oracle::taxonomy_decoder::decode_taxonomy;

// ── [Local decoding logic removed - now using laplace_oracle::taxonomy_decoder] ──

// ── Structures ──────────────────────────────────────────────────────────────

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TelemetryFrame {
    pub sync: [u8; 4],
    pub tick: u64,
    pub last_tick: u64,
    pub world_hash: [u8; 32],
    pub pop: u32,
    pub tech_mask: [u64; 4],
    pub apex_species_mask: u64,
    pub biomass: [u64; 16],
    pub water: [u64; 16],
    pub temperature: [u64; 16],
    pub structure: [u64; 16],
    pub particle: [u64; 16],
    pub pressure: [u64; 16],
    pub microbiome: [u64; 16],
    pub memetics: [u64; 1024],
    pub signature: [u8; 64],
}

const FRAME_SIZE: usize = 9248;

pub struct History {
    pub data: [u64; 256],
    pub head: usize,
}

impl History {
    pub fn new() -> Self { Self { data: [0; 256], head: 0 } }
    pub fn push(&mut self, val: u64) {
        self.data[self.head] = val;
        self.head = (self.head + 1) % 256;
    }
    pub fn values(&self) -> [u64; 256] {
        let mut v = [0u64; 256];
        for i in 0..256 { v[i] = self.data[(self.head + i) % 256]; }
        v
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
enum InputMode { #[default] Normal, Command }

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
enum VisualLayer {
    #[default] Biomass,
    Water,
    Temperature,
    Structure,
    Particle,
    Pressure,
    Microbiome,
    Memetics,
}

impl VisualLayer {
    fn next(&self) -> Self {
        match self {
            Self::Biomass => Self::Water,
            Self::Water => Self::Temperature,
            Self::Temperature => Self::Structure,
            Self::Structure => Self::Particle,
            Self::Particle => Self::Pressure,
            Self::Pressure => Self::Microbiome,
            Self::Microbiome => Self::Memetics,
            Self::Memetics => Self::Biomass,
        }
    }
}

struct TuiState {
    mode: InputMode,
    command_buffer: String,
    primary_layer: VisualLayer,
    reference_layer: Option<VisualLayer>,
    cursor_pos: (u8, u8),
    last_tick: u64,
    dropped_frames: u64,
    last_command_time: Option<SystemTime>,
}

enum RenderEvent { Telemetry(Box<TelemetryFrame>, bool) }

// ── Logic ───────────────────────────────────────────────────────────────────

fn parse_and_dispatch_command(state: &mut TuiState) {
    let cmd = state.command_buffer.trim();
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return; }

    let miracle = match parts[0] {
        "/genesis" => {
            let mask = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let (x, y) = if parts.len() >= 4 {
                (parts[2].parse::<u8>().unwrap_or(state.cursor_pos.0), parts[3].parse::<u8>().unwrap_or(state.cursor_pos.1))
            } else {
                state.cursor_pos
            };
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Genesis as u8,
                target_x: x,
                target_y: y,
                radius: 1,
                payload: mask,
            })
        }
        "/fire" => {
            let radius = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Fire as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius,
                payload: 0,
            })
        }
        "/rain" => {
            let radius = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Rain as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius,
                payload: 0,
            })
        }
        "/build" => {
            let radius = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Build as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius,
                payload: 0,
            })
        }
        "/flood" => {
            let radius = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Flood as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius,
                payload: 0,
            })
        }
        "/drought" => {
            let radius = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Drought as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius,
                payload: 0,
            })
        }
        "/infect" => {
            let hash = parts.get(1).and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(0);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Infect as u8,
                target_x: state.cursor_pos.0,
                target_y: state.cursor_pos.1,
                radius: 1,
                payload: hash,
            })
        }
        "/pause" => Some(MiracleCommand {
            nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
            miracle_type: MiracleType::Pause as u8,
            target_x: 0, target_y: 0, radius: 0, payload: 0,
        }),
        "/play" => Some(MiracleCommand {
            nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
            miracle_type: MiracleType::Play as u8,
            target_x: 0, target_y: 0, radius: 0, payload: 0,
        }),
        "/speed" => {
            let ms = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(16);
            Some(MiracleCommand {
                nonce: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64,
                miracle_type: MiracleType::Speed as u8,
                target_x: 0, target_y: 0, radius: 0, payload: ms,
            })
        }
        _ => None,
    };

    if let Some(m) = miracle {
        if let Ok(mut f) = OpenOptions::new().write(true).open("miracles.db") {
            let _ = f.write_all(&m.to_bytes());
            let _ = f.flush();
            state.last_command_time = Some(SystemTime::now());
        }
    }

    state.command_buffer.clear();
    state.mode = InputMode::Normal;
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Panopticon TUI: Laplace Oracle Visualization");
        println!("");
        println!("Usage: laplace-tui [PUBLIC_KEY_B64] [--help]");
        println!("");
        println!("Options:");
        println!("  --help, -h         Print this help message");
        println!("");
        println!("Note: If PUBLIC_KEY_B64 is not provided, it attempts to read /tmp/oracle.pub.");
        return Ok(());
    }

    setup_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    ctrlc::set_handler(move || {
        let _ = disable_raw_mode();
        let mut so = io::stdout();
        let _ = execute!(so, LeaveAlternateScreen);
        std::process::exit(0);
    }).unwrap();

    terminal.clear()?;

    let pk_str = if args.len() >= 2 && !args[1].starts_with('-') {
        args[1].clone()
    } else if let Ok(s) = std::fs::read_to_string("/tmp/oracle.pub") {
        s
    } else {
        cleanup_terminal(&mut terminal)?;
        eprintln!("Identity not found. Is the Oracle running?");
        std::process::exit(1);
    };

    let pk_bytes = base64::engine::general_purpose::STANDARD.decode(pk_str.trim())?;
    let vk = VerifyingKey::from_bytes(&pk_bytes.try_into().unwrap_or([0u8; 32]))?;

    let (tx, rx_ui) = bounded(256);

    // Ingest Thread
    thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut buf = [0u8; FRAME_SIZE];
        let sync = [0xAA, 0xBB, 0xCC, 0xDD];
        loop {
            if stdin.read_exact(&mut buf).is_err() { break; }
            if buf[0..4] == sync {
                let mut hash = [0u8; 32]; hash.copy_from_slice(&buf[20..52]);
                let mut tech = [0u64; 4];
                for i in 0..4 { tech[i] = u64::from_le_bytes(buf[56+i*8..64+i*8].try_into().unwrap()); }
                let (biomass, water, temperature, structure, particle, pressure, microbiome, memetics) = unpack_env(&buf);
                let frame = TelemetryFrame {
                    sync: [0xAA, 0xBB, 0xCC, 0xDD],
                    tick: u64::from_le_bytes(buf[4..12].try_into().unwrap()),
                    last_tick: u64::from_le_bytes(buf[12..20].try_into().unwrap()),
                    world_hash: hash,
                    pop: u32::from_le_bytes(buf[52..56].try_into().unwrap()),
                    tech_mask: tech,
                    apex_species_mask: u64::from_le_bytes(buf[88..96].try_into().unwrap()),
                    biomass, water, temperature, structure, particle, pressure, microbiome, memetics,
                    signature: buf[9184..9248].try_into().unwrap(),
                };
                let valid = vk.verify(&buf[4..9184], &Signature::from_bytes(&frame.signature)).is_ok();
                if valid { let _ = tx.send(RenderEvent::Telemetry(Box::new(frame), valid)); }
            }
        }
    });

    let mut history = History::new();
    let mut last: Option<(TelemetryFrame, bool)> = None;
    let mut state = TuiState {
        mode: InputMode::Normal,
        command_buffer: String::new(),
        primary_layer: VisualLayer::Biomass,
        reference_layer: None,
        cursor_pos: (32, 8),
        last_tick: 0,
        dropped_frames: 0,
        last_command_time: None,
    };

    loop {
        terminal.draw(|f| ui(f, &last, &history, &state))?;
        while let Ok(RenderEvent::Telemetry(fb, v)) = rx_ui.try_recv() {
            if state.last_tick != 0 && fb.tick != state.last_tick + 1 {
                state.dropped_frames += fb.tick.saturating_sub(state.last_tick).saturating_sub(1);
            }
            state.last_tick = fb.tick;
            history.push(fb.pop as u64);
            last = Some((*fb, v));
        }
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release { continue; }
                match state.mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(':') => state.mode = InputMode::Command,
                        KeyCode::Char(' ') => {
                            if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                state.reference_layer = match state.reference_layer {
                                    None => Some(VisualLayer::Biomass),
                                    Some(l) => {
                                        let next = l.next();
                                        if next == VisualLayer::Biomass { None } else { Some(next) }
                                    }
                                };
                            } else {
                                state.primary_layer = state.primary_layer.next();
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Left => state.cursor_pos.0 = state.cursor_pos.0.saturating_sub(1),
                        KeyCode::Char('l') | KeyCode::Right => state.cursor_pos.0 = (state.cursor_pos.0 + 1).min(63),
                        KeyCode::Char('k') | KeyCode::Up => state.cursor_pos.1 = state.cursor_pos.1.saturating_sub(1),
                        KeyCode::Char('j') | KeyCode::Down => state.cursor_pos.1 = (state.cursor_pos.1 + 1).min(15),
                        _ => {}
                    },
                    InputMode::Command => match key.code {
                        KeyCode::Enter => parse_and_dispatch_command(&mut state),
                        KeyCode::Esc => { state.mode = InputMode::Normal; state.command_buffer.clear(); }
                        KeyCode::Char(c) => state.command_buffer.push(c),
                        KeyCode::Backspace => { state.command_buffer.pop(); }
                        _ => {}
                    }
                }
            }
        }
    }
    cleanup_terminal(&mut terminal)?;
    Ok(())
}

fn unpack_env(buf: &[u8]) -> ([u64; 16], [u64; 16], [u64; 16], [u64; 16], [u64; 16], [u64; 16], [u64; 16], [u64; 1024]) {
    let mut out_bits = [[0u64; 16]; 7];
    for l in 0..7 {
        for i in 0..16 {
            let start = 96 + l * 128 + i * 8;
            out_bits[l][i] = u64::from_le_bytes(buf[start..start+8].try_into().unwrap());
        }
    }
    let mut out_memetics = [0u64; 1024];
    for i in 0..1024 {
        let start = 992 + i * 8;
        out_memetics[i] = u64::from_le_bytes(buf[start..start+8].try_into().unwrap());
    }
    (out_bits[0], out_bits[1], out_bits[2], out_bits[3], out_bits[4], out_bits[5], out_bits[6], out_memetics)
}

fn cleanup_terminal<B: Backend + Write>(t: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(t.backend_mut(), LeaveAlternateScreen)?;
    t.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, last: &Option<(TelemetryFrame, bool)>, history: &History, state: &TuiState) {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).split(f.area());
    let (t, h, s, st) = match last {
        Some((fr, v)) => (fr.tick, fr.world_hash, if *v { "VERIFIED" } else { "INVALID" }, if *v { Color::Green } else { Color::Red }),
        None => (0, [0u8; 32], "WAITING", Color::Yellow),
    };
    let header = Line::from(vec![
        Span::styled("Tick: ", Style::default().fg(Color::Gray)), Span::styled(t.to_string(), Style::default().fg(Color::White).bold()),
        Span::raw(" | Hash: "), Span::styled(hex::encode(h), Style::default().fg(Color::White)),
        Span::raw(" | Status: "), Span::styled(s, Style::default().fg(st).bold()),
        Span::raw(" | Dropped: "), Span::styled(state.dropped_frames.to_string(), Style::default().fg(Color::Red)),
    ]);
    f.render_widget(Paragraph::new(header).bg(Color::Black), chunks[0]);
    let body = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(chunks[1]);
    let left = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(body[0]);
    
    let pop_raw = history.values();
    let mut pop_data = [(0.0, 0.0); 256];
    let (mut min, mut max) = (f64::MAX, f64::MIN);
    for (i, &v) in pop_raw.iter().enumerate() {
        let fv = v as f64; pop_data[i] = (i as f64, fv);
        if fv < min { min = fv; } if fv > max { max = fv; }
    }
    let bounds = if (max - min).abs() < 1.0 { [min - 1.0, max + 1.0] } else { [min, max] };
    let chart = Chart::new(vec![Dataset::default().marker(symbols::Marker::Braille).graph_type(GraphType::Line).style(Color::Cyan).data(&pop_data)])
        .block(Block::default().title(" Population ").borders(Borders::ALL).bg(Color::Black))
        .x_axis(Axis::default().bounds([0.0, 255.0])).y_axis(Axis::default().bounds(bounds));
    f.render_widget(chart, left[0]);

    let apex = match last { Some((fr, _)) => decode_taxonomy(fr.apex_species_mask), None => "...".into() };
    f.render_widget(Paragraph::new(apex).wrap(ratatui::widgets::Wrap { trim: false }).block(Block::default().title(" Apex Species ").borders(Borders::ALL).bg(Color::Black)), left[1]);

    let bb = match last {
        Some((fr, _)) => {
            let mut s = String::with_capacity(1024 + 16);
            let p_layer = state.primary_layer;
            let r_layer = state.reference_layer;

            for y in 0..16 {
                for x in 0..64 {
                    if (x as u8, y as u8) == state.cursor_pos {
                        s.push('+');
                        continue;
                    }

                    let p_bit = if p_layer == VisualLayer::Memetics {
                        fr.memetics[y * 64 + x] != 0
                    } else {
                        let l = match p_layer {
                            VisualLayer::Biomass => fr.biomass,
                            VisualLayer::Water => fr.water,
                            VisualLayer::Temperature => fr.temperature,
                            VisualLayer::Structure => fr.structure,
                            VisualLayer::Particle => fr.particle,
                            VisualLayer::Pressure => fr.pressure,
                            VisualLayer::Microbiome => fr.microbiome,
                            _ => [0u64; 16],
                        };
                        (l[y] >> x) & 1 == 1
                    };

                    if p_bit {
                        if p_layer == VisualLayer::Memetics {
                            let hash = fr.memetics[y * 64 + x];
                            let chars = ['$', '@', '&', '#', '%', '?', '!', '*', '¤', '§', '¶', 'Δ', 'Ω', 'Ψ', 'Φ'];
                            let idx = (hash % chars.len() as u64) as usize;
                            s.push(chars[idx]);
                        } else {
                            s.push('█');
                        }
                    } else if let Some(ref_l) = r_layer {
                        let r_bit = if ref_l == VisualLayer::Memetics {
                            fr.memetics[y * 64 + x] != 0
                        } else {
                            let l = match ref_l {
                                VisualLayer::Biomass => fr.biomass,
                                VisualLayer::Water => fr.water,
                                VisualLayer::Temperature => fr.temperature,
                                VisualLayer::Structure => fr.structure,
                                VisualLayer::Particle => fr.particle,
                                VisualLayer::Pressure => fr.pressure,
                                VisualLayer::Microbiome => fr.microbiome,
                                _ => [0u64; 16],
                            };
                            (l[y] >> x) & 1 == 1
                        };
                        if r_bit {
                            s.push('░');
                        } else {
                            s.push(' ');
                        }
                    } else {
                        s.push(' ');
                    }
                }
                s.push('\n');
            }
            s
        }
        None => "...".into()
    };
    let title = if let Some(rl) = state.reference_layer {
        format!(" Bitboard [{:?} / {:?}] ", state.primary_layer, rl)
    } else {
        format!(" Bitboard [{:?}] ", state.primary_layer)
    };
    f.render_widget(Paragraph::new(bb).block(Block::default().title(title).borders(Borders::ALL).bg(Color::Black)), body[1]);

    let coords = format!(" CURSOR: ({:02}, {:02}) ", state.cursor_pos.0, state.cursor_pos.1);
    let bar = match state.mode {
        InputMode::Normal => Line::from(vec![
            Span::styled("-- NORMAL -- ", Style::default().fg(Color::Yellow).bold()),
            Span::styled(coords, Style::default().fg(Color::Cyan)),
            Span::styled("(Press ':')", Style::default().fg(Color::Gray))
        ]),
        InputMode::Command => Line::from(vec![
            Span::styled(":", Style::default().fg(Color::White).bold()),
            Span::raw(&state.command_buffer)
        ]),
    };
    
    // Command Acknowledgement overlay
    let bar = if let Some(last_time) = state.last_command_time {
        if last_time.elapsed().map(|e| e.as_secs_f32() < 1.0).unwrap_or(false) {
            let mut spans = bar.spans.clone();
            spans.push(Span::styled(" | ", Style::default().fg(Color::Gray)));
            spans.push(Span::styled("COMMAND ACKNOWLEDGED", Style::default().fg(Color::Green).bold()));
            Line::from(spans)
        } else {
            bar
        }
    } else {
        bar
    };

    f.render_widget(Paragraph::new(bar).bg(Color::Black), chunks[2]);
}

fn setup_panic_hook() {
    let old = panic::take_hook();
    panic::set_hook(Box::new(move |i| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        old(i);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_genesis() {
        let mut state = TuiState {
            mode: InputMode::Command,
            command_buffer: "/genesis 0x1234 10 20".into(),
            primary_layer: VisualLayer::Biomass,
            reference_layer: None,
            cursor_pos: (32, 8),
            last_tick: 0,
            dropped_frames: 0,
            last_command_time: None,
        };
        parse_and_dispatch_command(&mut state);
        assert_eq!(state.mode, InputMode::Normal);
        assert!(state.command_buffer.is_empty());
    }

    #[test]
    fn test_parse_fire_cursor() {
        let mut state = TuiState {
            mode: InputMode::Command,
            command_buffer: "/fire 5".into(),
            primary_layer: VisualLayer::Biomass,
            reference_layer: None,
            cursor_pos: (12, 6),
            last_tick: 0,
            dropped_frames: 0,
            last_command_time: None,
        };
        parse_and_dispatch_command(&mut state);
        assert_eq!(state.mode, InputMode::Normal);
        assert!(state.command_buffer.is_empty());
    }
}
