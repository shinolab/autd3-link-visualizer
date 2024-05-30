mod null;
#[cfg(feature = "plotters")]
mod plotters;
#[cfg(feature = "python")]
mod python;

use crate::error::VisualizerError;

use autd3_driver::{defined::Complex, geometry::Geometry};


pub trait Backend: Send + Sync {
    type PlotConfig;

    fn new() -> Self;

    fn initialize(&mut self) -> Result<(), VisualizerError>;

    fn plot_1d(
        observe_points: Vec<f64>,
        acoustic_pressures: Vec<Complex>,
        resolution: f64,
        x_label: &str,
        config: Self::PlotConfig,
    ) -> Result<(), VisualizerError>;

    #[allow(clippy::too_many_arguments)]
    fn plot_2d(
        observe_x: Vec<f64>,
        observe_y: Vec<f64>,
        acoustic_pressures: Vec<Complex>,
        resolution: f64,
        x_label: &str,
        y_label: &str,
        config: Self::PlotConfig,
    ) -> Result<(), VisualizerError>;

    fn plot_modulation(
        modulation: Vec<f64>,
        config: Self::PlotConfig,
    ) -> Result<(), VisualizerError>;

    fn plot_phase(
        config: Self::PlotConfig,
        geometry: &Geometry,
        phases: Vec<f64>,
    ) -> Result<(), VisualizerError>;
}

#[cfg(feature = "plotters")]
pub use self::plotters::*;
pub use null::*;
#[cfg(feature = "python")]
pub use python::*;
