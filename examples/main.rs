use std::path::Path;

use anyhow::Result;

use autd3::prelude::*;
use autd3_link_visualizer::*;

#[cfg(not(feature = "python"))]
use PlotConfig as Config;
#[cfg(feature = "python")]
use PyPlotConfig as Config;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "python")]
    let link = Visualizer::python();
    #[cfg(not(feature = "python"))]
    let link = Visualizer::plotters();

    #[cfg(feature = "gpu")]
    let link = link.with_gpu(-1);

    let mut autd = Controller::builder([AUTD3::new(Vector3::zeros())])
        .open(link)
        .await?;

    let center = autd.geometry().center() + Vector3::new(0., 0., 150.0 * mm);

    let g = Focus::new(center);
    let m = Square::new(150. * Hz);

    autd.send((m, g)).await?;

    autd.link().plot_phase(
        Config {
            fname: Path::new("phase.png").into(),
            ..Config::default()
        },
        Segment::S0,
        0,
    )?;
    autd.link().plot_field(
        Config {
            fname: Path::new("x.png").into(),
            ..Config::default()
        },
        PlotRange {
            x_range: center.x - 50.0..center.x + 50.0,
            y_range: center.y..center.y,
            z_range: center.z..center.z,
            resolution: 1.,
        },
        Segment::S0,
        0,
    )?;
    autd.link().plot_field(
        Config {
            fname: Path::new("xy.png").into(),
            ..Config::default()
        },
        PlotRange {
            x_range: center.x - 20.0..center.x + 20.0,
            y_range: center.y - 30.0..center.y + 30.0,
            z_range: center.z..center.z,
            resolution: 1.,
        },
        Segment::S0,
        0,
    )?;
    autd.link().plot_field(
        Config {
            fname: Path::new("yz.png").into(),
            ..Config::default()
        },
        PlotRange {
            x_range: center.x..center.x,
            y_range: center.y - 30.0..center.y + 30.0,
            z_range: 0.0..center.z + 50.0,
            resolution: 2.,
        },
        Segment::S0,
        0,
    )?;
    autd.link().plot_field(
        Config {
            fname: Path::new("zx.png").into(),
            ticks_step: 20.,
            ..Config::default()
        },
        PlotRange {
            x_range: center.x - 30.0..center.x + 30.0,
            y_range: center.y..center.y,
            z_range: 0.0..center.z + 50.0,
            resolution: 2.,
        },
        Segment::S0,
        0,
    )?;

    autd.link().plot_modulation(
        Config {
            fname: Path::new("mod.png").into(),
            ..Config::default()
        },
        Segment::S0,
    )?;

    // Calculate acoustic pressure without plotting
    let p = autd.link().calc_field(&[center], Segment::S0, 0)?;
    println!(
        "Acoustic pressure at ({}, {}, {}) = {} [Pa]",
        center.x,
        center.y,
        center.z,
        p[0].norm()
    );

    autd.close().await?;

    Ok(())
}
