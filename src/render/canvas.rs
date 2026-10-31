use tiny_skia::{Pixmap, Paint, PathBuilder, Stroke, Transform, FillRule};
use crate::config::{Color, Rect};
use crate::render::font::FontPool;

pub struct Canvas {
    pub pixmap: Pixmap,
    pub fonts: FontPool,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let pixmap = Pixmap::new(width, height)?;
        Some(Self { pixmap, fonts: FontPool::new() })
    }

    pub fn width(&self)  -> u32 { self.pixmap.width() }
    pub fn height(&self) -> u32 { self.pixmap.height() }
    pub fn data(&self)   -> &[u8] { self.pixmap.data() }
    pub fn data_mut(&mut self) -> &mut [u8] { self.pixmap.data_mut() }

    pub fn fill(&mut self, color: Color) {
        self.pixmap.fill(color.to_skia());
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        if let Some(r) = tiny_skia::Rect::from_xywh(
            rect.x as f32, rect.y as f32, rect.width as f32, rect.height as f32,
        ) {
            self.pixmap.fill_rect(r, &paint, Transform::identity(), None);
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, line_width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = false;
        let mut stroke = Stroke::default();
        stroke.width = line_width;
        let hw = line_width / 2.0;
        if let Some(r) = tiny_skia::Rect::from_xywh(
            rect.x as f32 + hw,
            rect.y as f32 + hw,
            (rect.width as f32 - line_width).max(0.0),
            (rect.height as f32 - line_width).max(0.0),
        ) {
            let mut pb = PathBuilder::new();
            pb.push_rect(r);
            if let Some(path) = pb.finish() {
                self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    pub fn fill_rect_gradient_h(&mut self, rect: Rect, color_left: Color, color_right: Color) {
        use tiny_skia::{LinearGradient, GradientStop, SpreadMode, Point};
        let Some(shader) = LinearGradient::new(
            Point::from_xy(rect.x as f32, 0.0),
            Point::from_xy((rect.x + rect.width as i32) as f32, 0.0),
            vec![
                GradientStop::new(0.0, color_left.to_skia()),
                GradientStop::new(1.0, color_right.to_skia()),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ) else { return };

        let mut paint = Paint::default();
        paint.shader = shader;
        if let Some(r) = tiny_skia::Rect::from_xywh(
            rect.x as f32, rect.y as f32, rect.width as f32, rect.height as f32,
        ) {
            self.pixmap.fill_rect(r, &paint, Transform::identity(), None);
        }
    }

    pub fn stroke_polyline(&mut self, points: &[(f32, f32)], color: Color, line_width: f32) {
        if points.len() < 2 { return; }
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        let mut stroke = Stroke::default();
        stroke.width = line_width;
        let mut pb = PathBuilder::new();
        pb.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] { pb.line_to(x, y); }
        if let Some(path) = pb.finish() {
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    pub fn fill_area_under_polyline(&mut self, points: &[(f32, f32)], base_y: f32, color: Color) {
        if points.len() < 2 { return; }
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        let mut pb = PathBuilder::new();
        pb.move_to(points[0].0, base_y);
        for &(x, y) in points { pb.line_to(x, y); }
        pb.line_to(points.last().unwrap().0, base_y);
        pb.close();
        if let Some(path) = pb.finish() {
            self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    pub fn fill_arc(
        &mut self,
        cx: f32, cy: f32,
        r_outer: f32, r_inner: f32,
        start_angle: f32, sweep_angle: f32,
        color: Color,
    ) {
        if sweep_angle.abs() < 0.001 { return; }
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        if let Some(path) = arc_path(cx, cy, r_outer, r_inner, start_angle, sweep_angle) {
            self.pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
        }
    }

    /// Draw text at (x, y) where y is the top of the text area.
    pub fn draw_text(
        &mut self,
        text: &str, x: f32, y: f32,
        font_name: &str, size: f32,
        color: Color, align: &str,
    ) {
        let text_w = self.fonts.measure_width(text, font_name, size);
        let draw_x = match align {
            "center" => x - text_w / 2.0,
            "right"  => x - text_w,
            _        => x,
        };
        self.fonts.draw_text(&mut self.pixmap, text, draw_x, y, font_name, size, color);
    }

    pub fn measure_text(&self, text: &str, font_name: &str, size: f32) -> f32 {
        self.fonts.measure_width(text, font_name, size)
    }

    /// Blit an RGBA image (straight alpha) into the canvas at dst_x, dst_y.
    pub fn draw_image_raw(&mut self, img: &image::DynamicImage, dst_x: i32, dst_y: i32) {
        let rgba = img.to_rgba8();
        let iw = rgba.width() as i32;
        let ih = rgba.height() as i32;
        let cw = self.pixmap.width() as i32;
        let ch = self.pixmap.height() as i32;
        let data = self.pixmap.data_mut();
        let stride = cw as usize * 4;
        for iy in 0..ih {
            for ix in 0..iw {
                let cx = dst_x + ix;
                let cy = dst_y + iy;
                if cx < 0 || cy < 0 || cx >= cw || cy >= ch { continue; }
                let src_px = rgba.get_pixel(ix as u32, iy as u32);
                let dst_off = cy as usize * stride + cx as usize * 4;
                let sa = src_px[3] as u32;
                if sa == 0 { continue; }
                if sa == 255 {
                    data[dst_off    ] = src_px[0];
                    data[dst_off + 1] = src_px[1];
                    data[dst_off + 2] = src_px[2];
                    data[dst_off + 3] = 255;
                } else {
                    let da = 255u32 - sa;
                    data[dst_off    ] = ((src_px[0] as u32 * sa + data[dst_off    ] as u32 * da) / 255) as u8;
                    data[dst_off + 1] = ((src_px[1] as u32 * sa + data[dst_off + 1] as u32 * da) / 255) as u8;
                    data[dst_off + 2] = ((src_px[2] as u32 * sa + data[dst_off + 2] as u32 * da) / 255) as u8;
                    data[dst_off + 3] = (sa + data[dst_off + 3] as u32 * da / 255).min(255) as u8;
                }
            }
        }
    }

    /// Blit a rendered tiny-skia pixmap (premultiplied RGBA) into this canvas.
    pub fn blit_pixmap(&mut self, src: &Pixmap, dst_x: i32, dst_y: i32) {
        let sw = src.width() as i32;
        let sh = src.height() as i32;
        let cw = self.pixmap.width() as i32;
        let ch = self.pixmap.height() as i32;
        let src_data = src.data();
        let dst_data = self.pixmap.data_mut();
        let dst_stride = cw as usize * 4;
        let src_stride = sw as usize * 4;
        for sy in 0..sh {
            for sx in 0..sw {
                let dx = dst_x + sx;
                let dy = dst_y + sy;
                if dx < 0 || dy < 0 || dx >= cw || dy >= ch { continue; }
                let si = sy as usize * src_stride + sx as usize * 4;
                let di = dy as usize * dst_stride + dx as usize * 4;
                let sa = src_data[si + 3] as u32;
                if sa == 0 { continue; }
                // Premultiplied source alpha compositing
                let ia = 255 - sa;
                dst_data[di    ] = (src_data[si    ] as u32 + dst_data[di    ] as u32 * ia / 255).min(255) as u8;
                dst_data[di + 1] = (src_data[si + 1] as u32 + dst_data[di + 1] as u32 * ia / 255).min(255) as u8;
                dst_data[di + 2] = (src_data[si + 2] as u32 + dst_data[di + 2] as u32 * ia / 255).min(255) as u8;
                dst_data[di + 3] = (sa + dst_data[di + 3] as u32 * ia / 255).min(255) as u8;
            }
        }
    }
}

/// Build a path for a doughnut arc segment using cubic bezier approximation.
fn arc_path(
    cx: f32, cy: f32,
    r_outer: f32, r_inner: f32,
    start_rad: f32, sweep_rad: f32,
) -> Option<tiny_skia::Path> {
    let steps = ((sweep_rad.abs() / std::f32::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep_rad / steps as f32;
    // Bezier kappa for this step size: k = 4/3 * tan(step/4)
    let k = (4.0f32 / 3.0) * (step / 4.0).tan();

    let mut pb = PathBuilder::new();
    pb.move_to(cx + r_outer * start_rad.cos(), cy + r_outer * start_rad.sin());

    let mut angle = start_rad;
    for _ in 0..steps {
        let next = angle + step;
        let (cos_s, sin_s) = (angle.cos(), angle.sin());
        let (cos_e, sin_e) = (next.cos(), next.sin());
        // Tangent at start: (-sin, cos) scaled by k
        let cp1x = cx + r_outer * cos_s - k * r_outer * sin_s;
        let cp1y = cy + r_outer * sin_s + k * r_outer * cos_s;
        // Tangent at end (approach from behind): (+sin, -cos) scaled by k
        let cp2x = cx + r_outer * cos_e + k * r_outer * sin_e;
        let cp2y = cy + r_outer * sin_e - k * r_outer * cos_e;
        pb.cubic_to(cp1x, cp1y, cp2x, cp2y, cx + r_outer * cos_e, cy + r_outer * sin_e);
        angle = next;
    }

    // Connect outer arc end to inner arc start
    let end = start_rad + sweep_rad;
    pb.line_to(cx + r_inner * end.cos(), cy + r_inner * end.sin());

    // Inner arc in reverse direction
    let k_rev = -k;
    angle = end;
    for _ in 0..steps {
        let next = angle - step;
        let (cos_s, sin_s) = (angle.cos(), angle.sin());
        let (cos_e, sin_e) = (next.cos(), next.sin());
        let cp1x = cx + r_inner * cos_s - k_rev * r_inner * sin_s;
        let cp1y = cy + r_inner * sin_s + k_rev * r_inner * cos_s;
        let cp2x = cx + r_inner * cos_e + k_rev * r_inner * sin_e;
        let cp2y = cy + r_inner * sin_e - k_rev * r_inner * cos_e;
        pb.cubic_to(cp1x, cp1y, cp2x, cp2y, cx + r_inner * cos_e, cy + r_inner * sin_e);
        angle = next;
    }

    pb.close();
    pb.finish()
}
