use plotters::prelude::*;
use polars::prelude::*;

pub fn plot_candles(df: &DataFrame, out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let column = |name: &str| -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        Ok(df.column(name)?.f32()?.into_no_null_iter().collect())
    };

    let open = column("open")?;
    let close = column("close")?;
    let min = column("min")?;
    let max = column("max")?;
    let median = column("median")?;

    if open.is_empty() {
        return Err("no candle data to plot".into());
    }

    let (width, height) = (1600u32, 800u32);
    let root = SVGBackend::new(out_path, (width, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let min_price = min.iter().copied().fold(f32::MAX, f32::min);
    let max_price = max.iter().copied().fold(f32::MIN, f32::max);
    let padding = ((max_price - min_price) * 0.05).max(1.0);

    let mut chart = ChartBuilder::on(&root)
        .caption("Market Simulation Candlestick Chart", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0f32..open.len() as f32,
            (min_price - padding)..(max_price + padding),
        )?;

    chart
        .configure_mesh()
        .x_desc("Round")
        .y_desc("Price")
        .draw()?;

    let plot_width_px = width.saturating_sub(120) as f32;
    let candle_width = ((plot_width_px / open.len() as f32) * 0.6).max(1.0) as u32;

    chart.draw_series((0..open.len()).map(|i| {
        CandleStick::new(
            i as f32,
            open[i],
            max[i],
            min[i],
            close[i],
            GREEN.filled(),
            RED.filled(),
            candle_width,
        )
    }))?;

    chart
        .draw_series(LineSeries::new(
            median.iter().enumerate().map(|(i, &m)| (i as f32, m)),
            BLUE.stroke_width(2),
        ))?
        .label("median")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}
