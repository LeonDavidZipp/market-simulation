use plotters::prelude::*;
use polars::prelude::*;

fn format_price(y: f32) -> String {
    if y.abs() >= 1_000_000.0 {
        return format!("{:.2} Mio", y / 1_000_000.0);
    }
    let rounded = format!("{:.2}", y);
    rounded
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

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

    let scale = 3u32;
    let (width, height) = (1600 * scale, 800 * scale);
    let root = SVGBackend::new(out_path, (width, height)).into_drawing_area();
    root.fill(&WHITE)?;

    let min_price = min.iter().copied().fold(f32::MAX, f32::min);
    let max_price = max.iter().copied().fold(f32::MIN, f32::max);
    let padding = ((max_price - min_price) * 0.05).max(1.0);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Simulation Simulation Candlestick Chart",
            ("sans-serif", 30 * scale),
        )
        .margin(20 * scale)
        .x_label_area_size(40 * scale)
        .y_label_area_size(60 * scale)
        .build_cartesian_2d(
            0f32..open.len() as f32,
            (min_price - padding)..(max_price + padding),
        )?;

    chart
        .configure_mesh()
        .x_label_style(("sans-serif", 15 * scale))
        .y_label_style(("sans-serif", 15 * scale))
        .x_label_formatter(&|x| format!("{}", *x as i64))
        .y_label_formatter(&|y| format_price(*y))
        .x_desc("Step")
        .y_desc("Price")
        .draw()?;

    let plot_width_px = width.saturating_sub(120 * scale) as f32;
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
            BLUE.stroke_width(scale.div_ceil(2)),
        ))?
        .label("median")
        .legend(move |(x, y)| {
            PathElement::new(
                vec![(x, y), (x + 20, y)],
                BLUE.stroke_width(scale.div_ceil(2)),
            )
        });

    chart
        .configure_series_labels()
        .label_font(("sans-serif", 15 * scale))
        .legend_area_size(40 * scale)
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}
