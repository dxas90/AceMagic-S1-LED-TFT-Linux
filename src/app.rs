//! Application state and main draw loop.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, warn};

use crate::config::{split_update_region, AppConfig, Color, Rect, Screen, Theme};
use crate::hardware::lcd::{pixmap_to_rgb565, translate_rect_portrait, LcdDevice};
use crate::hardware::led::{LedCommand, LedDevice};

/// Messages sent over the LCD channel to the hardware worker thread.
pub enum LcdCommand {
    Heartbeat,
    SetOrientation(bool),
    PartialUpdate(Rect, Vec<u16>),
    FullRedraw(Vec<u16>),
}
use crate::render::canvas::Canvas;
use crate::render::widgets::Widget;
use crate::sensors::{SensorMap, SensorReading};

// ── AppState (shared across draw loop + API) ──────────────────────────────────

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub theme: RwLock<Theme>,
    pub sensors: Mutex<SensorMap>,
    pub canvas: Mutex<Canvas>,
    pub drawing: AtomicBool,
    pub redraw_want: AtomicU64,
    pub redraw_done: AtomicU64,
    pub screen_index: AtomicUsize,
    pub unsaved_changes: AtomicBool,
    pub lcd_tx: mpsc::Sender<LcdCommand>,
    pub led_tx: mpsc::Sender<LedCommand>,
}

impl AppState {
    pub async fn new(
        config: AppConfig,
        theme: Theme,
        sensors: SensorMap,
        lcd_tx: mpsc::Sender<LcdCommand>,
        led_tx: mpsc::Sender<LedCommand>,
    ) -> Arc<Self> {
        let (w, h) = (config.display.width, config.display.height);
        let canvas = Canvas::new(w, h).expect("failed to create canvas");
        Arc::new(Self {
            config: RwLock::new(config),
            theme: RwLock::new(theme),
            sensors: Mutex::new(sensors),
            canvas: Mutex::new(canvas),
            drawing: AtomicBool::new(false),
            redraw_want: AtomicU64::new(0),
            redraw_done: AtomicU64::new(0),
            screen_index: AtomicUsize::new(0),
            unsaved_changes: AtomicBool::new(false),
            lcd_tx,
            led_tx,
        })
    }

    /// Trigger a full LCD redraw on the next draw cycle.
    pub fn request_redraw(&self) {
        self.redraw_want.fetch_add(1, Ordering::SeqCst);
    }

    /// Get a PNG snapshot of the current canvas for the API.
    pub async fn canvas_png(&self) -> Option<Vec<u8>> {
        let canvas = self.canvas.lock().await;
        canvas.pixmap.encode_png().ok()
    }
}

// ── Draw loop ─────────────────────────────────────────────────────────────────

pub struct DrawLoop {
    state: Arc<AppState>,
    widget_cache: HashMap<u32, Box<dyn Widget>>,
    screen_entered_at: Instant,
    heartbeat_at: Instant,
}

impl DrawLoop {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            widget_cache: HashMap::new(),
            screen_entered_at: Instant::now(),
            heartbeat_at: Instant::now(),
        }
    }

    pub async fn run(mut self) {
        let poll_ms = self.state.config.read().await.display.poll_ms;
        let hb_ms = self.state.config.read().await.display.heartbeat_ms;
        let mut ticker = interval(Duration::from_millis(poll_ms));

        loop {
            ticker.tick().await;

            // Heartbeat
            if self.heartbeat_at.elapsed() >= Duration::from_millis(hb_ms) {
                let _ = self.state.lcd_tx.try_send(LcdCommand::Heartbeat);
                self.heartbeat_at = Instant::now();
            }

            self.tick().await;
        }
    }

    async fn tick(&mut self) {
        let config = self.state.config.read().await.clone();
        let theme = self.state.theme.read().await.clone();
        let is_portrait = theme.is_portrait();

        // Rotate screen if duration expired
        let screens: Vec<&Screen> = theme.screens.iter().filter(|s| s.enabled).collect();
        if screens.is_empty() {
            return;
        }
        let idx = self.state.screen_index.load(Ordering::Relaxed) % screens.len();
        let screen = screens[idx];

        if screen.duration > 0
            && self.screen_entered_at.elapsed() >= Duration::from_millis(screen.duration)
        {
            let next = (idx + 1) % screens.len();
            self.state.screen_index.store(next, Ordering::Relaxed);
            self.screen_entered_at = Instant::now();
            self.state.request_redraw();
            debug!("Rotating to screen {} ({})", next, screens[next].name);
        }

        // Sample sensors
        let poll_ms = config.display.poll_ms;
        let readings = self.sample_sensors(poll_ms).await;

        // Draw widgets onto canvas
        let force_redraw = self.state.redraw_want.load(Ordering::SeqCst)
            > self.state.redraw_done.load(Ordering::SeqCst);
        let dirty_rects = self
            .draw_screen(screen, &readings, force_redraw, is_portrait)
            .await;

        if force_redraw {
            self.state.redraw_done.fetch_add(1, Ordering::SeqCst);
        }

        if dirty_rects.is_empty() && !force_redraw {
            return;
        }

        // Apply per-screen LED config if present
        if let Some(ref led_cfg) = screen.led_config {
            let cmd = LedCommand {
                theme: led_cfg.theme,
                intensity: led_cfg.intensity,
                speed: led_cfg.speed,
            };
            let _ = self.state.led_tx.try_send(cmd);
        }

        // Send to LCD
        let canvas = self.state.canvas.lock().await;
        if force_redraw || config.display.refresh_ms == 0 {
            // Full redraw
            let pixels = pixmap_to_rgb565(canvas.data(), is_portrait);
            let _ = self.state.lcd_tx.try_send(LcdCommand::FullRedraw(pixels));
        } else {
            // Partial updates for dirty rects
            for raw_rect in dirty_rects {
                let phys_rect = if is_portrait {
                    translate_rect_portrait(raw_rect, config.display.height)
                } else {
                    raw_rect
                };
                // Extract pixels for this rect from the canvas
                for sub in split_update_region(phys_rect) {
                    let pixels = extract_rect_pixels(
                        canvas.data(),
                        canvas.width(),
                        canvas.height(),
                        sub,
                        is_portrait,
                    );
                    let _ = self
                        .state
                        .lcd_tx
                        .try_send(LcdCommand::PartialUpdate(sub, pixels));
                }
            }
        }
    }

    async fn sample_sensors(&mut self, poll_ms: u64) -> HashMap<String, SensorReading> {
        let mut sensors = self.state.sensors.lock().await;
        sensors
            .iter_mut()
            .map(|(name, sensor)| {
                let reading = sensor.sample(poll_ms);
                (name.clone(), reading)
            })
            .collect()
    }

    async fn draw_screen(
        &mut self,
        screen: &Screen,
        readings: &HashMap<String, SensorReading>,
        force_redraw: bool,
        _is_portrait: bool,
    ) -> Vec<Rect> {
        let mut canvas = self.state.canvas.lock().await;
        let mut dirty = Vec::new();

        // Background
        if force_redraw {
            if let Some(ref bg) = screen.background {
                let color = Color::from_hex(bg).unwrap_or(Color::BLACK);
                canvas.fill(color);
            } else {
                canvas.fill(Color::BLACK);
            }
        }

        for widget_cfg in &screen.widgets {
            let value = if widget_cfg.sensor {
                readings
                    .get(&widget_cfg.value)
                    .map(|r| r.format(&widget_cfg.format))
                    .unwrap_or_default()
            } else {
                widget_cfg.value.clone()
            };
            let (min, max) = if widget_cfg.sensor {
                readings
                    .get(&widget_cfg.value)
                    .map(|r| (r.min, r.max))
                    .unwrap_or((0.0, 100.0))
            } else {
                (0.0, 100.0)
            };

            let widget = self.widget_cache.entry(widget_cfg.id).or_insert_with(|| {
                crate::render::widgets::create_widget(&widget_cfg.name).unwrap_or_else(|| {
                    warn!(
                        "Unknown widget type: {}, falling back to text",
                        widget_cfg.name
                    );
                    Box::new(crate::render::widgets::text::TextWidget::new())
                })
            });

            let changed = widget.draw(&mut canvas, &value, min, max, widget_cfg);
            if changed || force_redraw {
                dirty.push(widget_cfg.rect);
            }
        }

        dirty
    }
}

// ── Hardware worker tasks ─────────────────────────────────────────────────────

/// Runs in a spawned_blocking thread, owns the LCD device.
pub fn lcd_worker(mut rx: mpsc::Receiver<LcdCommand>, device: LcdDevice) {
    while let Some(cmd) = rx.blocking_recv() {
        let result = match cmd {
            LcdCommand::Heartbeat => device.heartbeat(),
            LcdCommand::SetOrientation(portrait) => device.set_orientation(portrait),
            LcdCommand::PartialUpdate(rect, pixels) => device.partial_update(rect, &pixels),
            LcdCommand::FullRedraw(pixels) => device.full_redraw(&pixels),
        };
        if let Err(e) = result {
            warn!("LCD error: {e}");
        }
    }
}

/// Runs in a spawned_blocking thread, owns the LED device.
pub fn led_worker(mut rx: mpsc::Receiver<LedCommand>, mut device: LedDevice) {
    while let Some(cmd) = rx.blocking_recv() {
        if let Err(e) = device.send(cmd) {
            warn!("LED error: {e}");
        }
    }
}

/// Extract RGBA pixels for a sub-rect from the full canvas data, returning RGB565.
fn extract_rect_pixels(
    data: &[u8],
    canvas_w: u32,
    canvas_h: u32,
    rect: Rect,
    portrait: bool,
) -> Vec<u16> {
    use crate::hardware::lcd::rgba_to_rgb565_be;
    let stride = canvas_w as usize * 4;
    let mut pixels = Vec::with_capacity((rect.width * rect.height) as usize);

    for row in 0..rect.height {
        for col in 0..rect.width {
            let (cx, cy) = if portrait {
                // Portrait: physical (col, row) within rect maps to logical coords.
                // logical_x = (canvas_h - 1) - (rect.y + row)
                // logical_y = rect.x + col
                let lx = (canvas_h as i32 - 1 - rect.y - row as i32).max(0) as usize;
                let ly = (rect.x + col as i32).max(0) as usize;
                (lx, ly)
            } else {
                (
                    (rect.x as usize + col as usize),
                    (rect.y as usize + row as usize),
                )
            };
            let off = cy * stride + cx * 4;
            if off + 3 < data.len() {
                pixels.push(rgba_to_rgb565_be(data[off], data[off + 1], data[off + 2]));
            } else {
                pixels.push(0);
            }
        }
    }
    pixels
}

// ── Config loading ────────────────────────────────────────────────────────────

pub fn load_config(path: &str) -> crate::error::Result<AppConfig> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(crate::error::Error::Toml)
}

pub fn load_theme(path: &str) -> crate::error::Result<Theme> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(crate::error::Error::Json)
}

pub fn save_config(config: &AppConfig, path: &str) -> crate::error::Result<()> {
    use std::io::Write;
    let content =
        toml::to_string_pretty(config).map_err(|e| crate::error::Error::Config(e.to_string()))?;
    // Atomic write via tempfile in same directory
    let dir = std::path::Path::new(path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    tmp.persist(path)
        .map_err(|e| crate::error::Error::Io(e.error))?;
    Ok(())
}
