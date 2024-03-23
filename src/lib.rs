mod error;

#[cfg(feature = "gpu")]
mod gpu;

mod backend;

pub use backend::*;

#[cfg(feature = "plotters")]
pub mod colormap;

use std::{marker::PhantomData, time::Duration};

use autd3_driver::{
    acoustics::{
        directivity::{Directivity, Sphere},
        propagate,
    },
    common::{EmitIntensity, Phase, Segment},
    cpu::{RxMessage, TxDatagram},
    defined::Complex,
    error::AUTDInternalError,
    geometry::{Geometry, Vector3},
    link::{Link, LinkBuilder},
};
use autd3_firmware_emulator::CPUEmulator;

#[cfg(feature = "plotters")]
pub use scarlet::colormap::ListedColorMap;

use error::VisualizerError;

/// Link to monitoring the status of AUTD and acoustic field
pub struct Visualizer<D, B>
where
    D: Directivity,
    B: Backend,
{
    is_open: bool,
    timeout: Duration,
    cpus: Vec<CPUEmulator>,
    _d: PhantomData<D>,
    _b: PhantomData<B>,
    #[cfg(feature = "gpu")]
    gpu_compute: Option<gpu::FieldCompute>,
}

pub struct VisualizerBuilder<D, B>
where
    D: Directivity,
    B: Backend,
{
    backend: B,
    timeout: Duration,
    _d: PhantomData<D>,
    #[cfg(feature = "gpu")]
    gpu_idx: Option<i32>,
}

#[cfg_attr(feature = "async-trait", autd3_driver::async_trait)]
impl<D: Directivity, B: Backend> LinkBuilder for VisualizerBuilder<D, B> {
    type L = Visualizer<D, B>;

    #[allow(unused_mut)]
    async fn open(mut self, geometry: &Geometry) -> Result<Self::L, AUTDInternalError> {
        #[cfg(feature = "gpu")]
        let gpu_compute = if let Some(gpu_idx) = self.gpu_idx {
            Some(gpu::FieldCompute::new(gpu_idx)?)
        } else {
            None
        };

        let VisualizerBuilder {
            mut backend,
            timeout,
            ..
        } = self;

        backend.initialize()?;

        Ok(Self::L {
            is_open: true,
            timeout,
            cpus: geometry
                .iter()
                .enumerate()
                .map(|(i, dev)| {
                    let mut cpu = CPUEmulator::new(i, dev.num_transducers());
                    cpu.init();
                    cpu
                })
                .collect(),
            _d: PhantomData,
            _b: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_compute,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PlotRange {
    pub x_range: std::ops::Range<f64>,
    pub y_range: std::ops::Range<f64>,
    pub z_range: std::ops::Range<f64>,
    pub resolution: f64,
}

impl PlotRange {
    fn n(range: &std::ops::Range<f64>, resolution: f64) -> usize {
        ((range.end - range.start) / resolution).floor() as usize + 1
    }

    pub fn nx(&self) -> usize {
        Self::n(&self.x_range, self.resolution)
    }

    pub fn ny(&self) -> usize {
        Self::n(&self.y_range, self.resolution)
    }

    pub fn nz(&self) -> usize {
        Self::n(&self.z_range, self.resolution)
    }

    fn is_1d(&self) -> bool {
        matches!(
            (self.nx(), self.ny(), self.nz()),
            (_, 1, 1) | (1, _, 1) | (1, 1, _)
        )
    }

    fn is_2d(&self) -> bool {
        if self.is_1d() {
            return false;
        }
        matches!(
            (self.nx(), self.ny(), self.nz()),
            (1, _, _) | (_, 1, _) | (_, _, 1)
        )
    }

    fn observe(n: usize, start: f64, resolution: f64) -> Vec<f64> {
        (0..n).map(|i| start + resolution * i as f64).collect()
    }

    fn observe_x(&self) -> Vec<f64> {
        Self::observe(self.nx(), self.x_range.start, self.resolution)
    }

    fn observe_y(&self) -> Vec<f64> {
        Self::observe(self.ny(), self.y_range.start, self.resolution)
    }

    fn observe_z(&self) -> Vec<f64> {
        Self::observe(self.nz(), self.z_range.start, self.resolution)
    }

    pub fn observe_points(&self) -> Vec<Vector3> {
        match (self.nx(), self.ny(), self.nz()) {
            (_, 1, 1) => self
                .observe_x()
                .iter()
                .map(|&x| Vector3::new(x, self.y_range.start, self.z_range.start))
                .collect(),
            (1, _, 1) => self
                .observe_y()
                .iter()
                .map(|&y| Vector3::new(self.x_range.start, y, self.z_range.start))
                .collect(),
            (1, 1, _) => self
                .observe_z()
                .iter()
                .map(|&z| Vector3::new(self.x_range.start, self.y_range.start, z))
                .collect(),
            (_, _, 1) => itertools::iproduct!(self.observe_y(), self.observe_x())
                .map(|(y, x)| Vector3::new(x, y, self.z_range.start))
                .collect(),
            (_, 1, _) => itertools::iproduct!(self.observe_x(), self.observe_z())
                .map(|(x, z)| Vector3::new(x, self.y_range.start, z))
                .collect(),
            (1, _, _) => itertools::iproduct!(self.observe_z(), self.observe_y())
                .map(|(z, y)| Vector3::new(self.x_range.start, y, z))
                .collect(),
            (_, _, _) => itertools::iproduct!(self.observe_z(), self.observe_y(), self.observe_x())
                .map(|(z, y, x)| Vector3::new(x, y, z))
                .collect(),
        }
    }
}

impl Visualizer<Sphere, PlottersBackend> {
    pub fn builder() -> VisualizerBuilder<Sphere, PlottersBackend> {
        VisualizerBuilder {
            backend: PlottersBackend::new(),
            timeout: Duration::ZERO,
            _d: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_idx: None,
        }
    }
}

#[cfg(feature = "plotters")]
impl Visualizer<Sphere, PlottersBackend> {
    /// Constructor with Plotters backend
    pub fn plotters() -> VisualizerBuilder<Sphere, PlottersBackend> {
        Self::builder()
    }
}

#[cfg(feature = "python")]
impl Visualizer<Sphere, PythonBackend> {
    /// Constructor with Python backend
    pub fn python() -> VisualizerBuilder<Sphere, PythonBackend> {
        VisualizerBuilder {
            backend: PythonBackend::new(),
            timeout: Duration::ZERO,
            _d: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_idx: None,
        }
    }
}

impl Visualizer<Sphere, NullBackend> {
    /// Constructor with Null backend
    pub fn null() -> VisualizerBuilder<Sphere, NullBackend> {
        VisualizerBuilder {
            backend: NullBackend::new(),
            timeout: Duration::ZERO,
            _d: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_idx: None,
        }
    }
}

impl<D: Directivity, B: Backend> VisualizerBuilder<D, B> {
    /// Set directivity
    pub fn with_directivity<U: Directivity>(self) -> VisualizerBuilder<U, B> {
        VisualizerBuilder {
            backend: self.backend,
            timeout: self.timeout,
            _d: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_idx: self.gpu_idx,
        }
    }

    /// Set backend
    pub fn with_backend<U: Backend>(self) -> VisualizerBuilder<D, U> {
        VisualizerBuilder {
            backend: U::new(),
            timeout: self.timeout,
            _d: PhantomData,
            #[cfg(feature = "gpu")]
            gpu_idx: self.gpu_idx,
        }
    }
}

#[cfg(feature = "gpu")]
impl<D: Directivity, B: Backend> VisualizerBuilder<D, B> {
    /// Enable GPU acceleration
    pub fn with_gpu(self, gpu_idx: i32) -> VisualizerBuilder<D, B> {
        Self {
            gpu_idx: Some(gpu_idx),
            ..self
        }
    }
}

impl<D: Directivity, B: Backend> Visualizer<D, B> {
    /// Get phases of transducers
    ///
    /// # Arguments
    ///
    /// * `idx` - Index of STM. If you use Gain, this value should be 0.
    ///
    pub fn phases(&self, segment: Segment, idx: usize) -> Vec<Phase> {
        self.cpus
            .iter()
            .flat_map(|cpu| {
                cpu.fpga()
                    .drives(segment, idx)
                    .iter()
                    .map(|&d| d.phase())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Get intensities of transducers
    ///
    /// # Arguments
    ///
    /// * `idx` - Index of STM. If you use Gain, this value should be 0.
    ///
    pub fn intensities(&self, segment: Segment, idx: usize) -> Vec<EmitIntensity> {
        self.cpus
            .iter()
            .flat_map(|cpu| {
                cpu.fpga()
                    .drives(segment, idx)
                    .iter()
                    .map(|&d| d.intensity())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Get raw modulation data
    pub fn modulation(&self, segment: Segment) -> Vec<EmitIntensity> {
        self.cpus[0]
            .fpga()
            .modulation(segment)
            .into_iter()
            .collect()
    }

    /// Calculate acoustic field at specified points
    ///
    /// # Arguments
    ///
    /// * `observe_points` - Observe points iterator
    /// * `geometry` - Geometry
    /// * `idx` - Index of STM. If you use Gain, this value should be 0.
    ///
    pub fn calc_field<'a, I: IntoIterator<Item = &'a Vector3>>(
        &self,
        observe_points: I,
        geometry: &Geometry,
        segment: Segment,
        idx: usize,
    ) -> Result<Vec<Complex>, VisualizerError> {
        #[cfg(feature = "gpu")]
        {
            if let Some(gpu) = &self.gpu_compute {
                let source_drive = self
                    .cpus
                    .iter()
                    .enumerate()
                    .flat_map(|(i, cpu)| {
                        let dev = &geometry[i];
                        let sound_speed = dev.sound_speed;
                        cpu.fpga()
                            .drives(segment, idx)
                            .iter()
                            .zip(dev.iter().map(|t| t.wavenumber(sound_speed)))
                            .map(|(d, w)| {
                                let amp = d.intensity().value() as f32 / 255.0;
                                let phase = d.phase().radian() as f32;
                                [amp, phase, 0., w as f32]
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                return gpu.calc_field_of::<D, I>(observe_points, geometry, source_drive);
            }
        }
        Ok(observe_points
            .into_iter()
            .map(|target| {
                self.cpus
                    .iter()
                    .enumerate()
                    .fold(Complex::new(0., 0.), |acc, (i, cpu)| {
                        let sound_speed = geometry[i].sound_speed;
                        let drives = cpu.fpga().drives(segment, idx);
                        acc + geometry[i].iter().zip(drives.iter()).fold(
                            Complex::new(0., 0.),
                            |acc, (t, d)| {
                                let amp = d.intensity().value() as f64 / 255.0;
                                let phase = d.phase().radian();
                                acc + propagate::<D>(t, 0.0, sound_speed, target)
                                    * Complex::from_polar(amp, phase)
                            },
                        )
                    })
            })
            .collect())
    }

    /// Plot acoustic field
    ///
    /// # Arguments
    ///
    /// * `config` - Plot configuration
    /// * `range` - Plot range
    /// * `geometry` - Geometry
    /// * `idx` - Index of STM. If you use Gain, this value should be 0.
    ///
    pub fn plot_field(
        &self,
        config: B::PlotConfig,
        range: PlotRange,
        geometry: &Geometry,
        segment: Segment,
        idx: usize,
    ) -> Result<(), VisualizerError> {
        let observe_points = range.observe_points();
        let acoustic_pressures = self.calc_field(&observe_points, geometry, segment, idx)?;
        if range.is_1d() {
            let (observe, label) = match (range.nx(), range.ny(), range.nz()) {
                (_, 1, 1) => (range.observe_x(), "x [mm]"),
                (1, _, 1) => (range.observe_y(), "y [mm]"),
                (1, 1, _) => (range.observe_z(), "z [mm]"),
                _ => unreachable!(),
            };
            B::plot_1d(observe, acoustic_pressures, range.resolution, label, config)
        } else if range.is_2d() {
            let (observe_x, x_label) = match (range.nx(), range.ny(), range.nz()) {
                (_, _, 1) => (range.observe_x(), "x [mm]"),
                (1, _, _) => (range.observe_y(), "y [mm]"),
                (_, 1, _) => (range.observe_z(), "z [mm]"),
                _ => unreachable!(),
            };
            let (observe_y, y_label) = match (range.nx(), range.ny(), range.nz()) {
                (_, _, 1) => (range.observe_y(), "y [mm]"),
                (1, _, _) => (range.observe_z(), "z [mm]"),
                (_, 1, _) => (range.observe_x(), "x [mm]"),
                _ => unreachable!(),
            };
            B::plot_2d(
                observe_x,
                observe_y,
                acoustic_pressures,
                range.resolution,
                x_label,
                y_label,
                config,
            )
        } else {
            Err(VisualizerError::InvalidPlotRange)
        }
    }

    /// Plot transducers with phases
    ///
    /// # Arguments
    ///
    /// * `config` - Plot configuration
    /// * `geometry` - Geometry
    /// * `idx` - Index of STM. If you use Gain, this value should be 0.
    ///
    pub fn plot_phase(
        &self,
        config: B::PlotConfig,
        geometry: &Geometry,
        segment: Segment,
        idx: usize,
    ) -> Result<(), VisualizerError> {
        let phases = self
            .phases(segment, idx)
            .iter()
            .map(Phase::radian)
            .collect();
        B::plot_phase(config, geometry, phases)
    }

    /// Plot modulation data
    pub fn plot_modulation(
        &self,
        config: B::PlotConfig,
        segment: Segment,
    ) -> Result<(), VisualizerError> {
        let m = self
            .modulation(segment)
            .iter()
            .map(|v| v.value() as f64 / 255.0)
            .collect::<Vec<_>>();
        B::plot_modulation(m, config)?;
        Ok(())
    }
}

#[cfg_attr(feature = "async-trait", autd3_driver::async_trait)]
impl<D: Directivity, B: Backend> Link for Visualizer<D, B> {
    async fn close(&mut self) -> Result<(), AUTDInternalError> {
        if !self.is_open {
            return Ok(());
        }

        self.is_open = false;
        Ok(())
    }

    async fn send(&mut self, tx: &TxDatagram) -> Result<bool, AUTDInternalError> {
        if !self.is_open {
            return Ok(false);
        }

        self.cpus.iter_mut().for_each(|cpu| {
            cpu.send(tx);
        });

        Ok(true)
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<bool, AUTDInternalError> {
        if !self.is_open {
            return Ok(false);
        }

        self.cpus.iter_mut().for_each(|cpu| {
            cpu.update();
            rx[cpu.idx()] = RxMessage::new(cpu.ack(), cpu.rx_data());
        });

        Ok(true)
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
