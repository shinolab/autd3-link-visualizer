use crate::Backend;


pub struct NullBackend {}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct NullPlotConfig {}

impl Backend for NullBackend {
    type PlotConfig = NullPlotConfig;

    fn new() -> Self {
        Self {}
    }

    fn initialize(&mut self) -> Result<(), crate::error::VisualizerError> {
        Ok(())
    }

    fn plot_1d(
        _observe_points: Vec<f64>,
        _acoustic_pressures: Vec<autd3_driver::defined::Complex>,
        _resolution: f64,
        _x_label: &str,
        _config: Self::PlotConfig,
    ) -> Result<(), crate::error::VisualizerError> {
        Err(crate::error::VisualizerError::NotSupported)
    }

    fn plot_2d(
        _observe_x: Vec<f64>,
        _observe_y: Vec<f64>,
        _acoustic_pressures: Vec<autd3_driver::defined::Complex>,
        _resolution: f64,
        _x_label: &str,
        _y_label: &str,
        _config: Self::PlotConfig,
    ) -> Result<(), crate::error::VisualizerError> {
        Err(crate::error::VisualizerError::NotSupported)
    }

    fn plot_modulation(
        _modulation: Vec<f64>,
        _config: Self::PlotConfig,
    ) -> Result<(), crate::error::VisualizerError> {
        Err(crate::error::VisualizerError::NotSupported)
    }

    fn plot_phase(
        _config: Self::PlotConfig,
        _geometry: &autd3_driver::geometry::Geometry,
        _phases: Vec<f64>,
    ) -> Result<(), crate::error::VisualizerError> {
        Err(crate::error::VisualizerError::NotSupported)
    }
}
