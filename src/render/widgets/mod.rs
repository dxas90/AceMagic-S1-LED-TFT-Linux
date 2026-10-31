use crate::config::WidgetConfig;
use super::canvas::Canvas;

pub trait Widget: Send + Sync {
    fn name(&self) -> &str;
    /// Draw into `canvas`. Returns true if the canvas was modified.
    fn draw(
        &mut self,
        canvas: &mut Canvas,
        value: &str,
        min: f64,
        max: f64,
        config: &WidgetConfig,
    ) -> bool;
}

pub mod bar_chart;
pub mod custom_bar;
pub mod doughnut;
pub mod iconify;
pub mod image_widget;
pub mod line_chart;
pub mod text;
pub mod weather_icon;

pub fn create_widget(name: &str) -> Option<Box<dyn Widget>> {
    match name {
        "text"          => Some(Box::new(text::TextWidget::new())),
        "custom_bar"    => Some(Box::new(custom_bar::CustomBarWidget::new())),
        "bar_chart"     => Some(Box::new(bar_chart::BarChartWidget::new())),
        "line_chart"    => Some(Box::new(line_chart::LineChartWidget::new())),
        "doughnut"      => Some(Box::new(doughnut::DoughnutWidget::new())),
        "image"         => Some(Box::new(image_widget::ImageWidget::new())),
        "iconify"       => Some(Box::new(iconify::IconifyWidget::new())),
        "weather_icon"  => Some(Box::new(weather_icon::WeatherIconWidget::new())),
        _ => None,
    }
}
