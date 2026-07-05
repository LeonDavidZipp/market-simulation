use crate::order_book::EmptyDataError;
use crate::order_book::{CandleData, Order, OrderBook};
use polars::error::PolarsError;
use polars::prelude::{CsvWriter, DataFrame, ParquetWriter, SerWriter, df};
use rand::RngExt;
use rand::rng;
use rand_distr::{Distribution, Normal, NormalError};
use std::io::Write;

#[derive(Debug)]
pub enum MarketError {
    EmptyData(EmptyDataError),
    InvalidDistribution(NormalError),
}

impl From<EmptyDataError> for MarketError {
    fn from(e: EmptyDataError) -> Self {
        MarketError::EmptyData(e)
    }
}

impl From<NormalError> for MarketError {
    fn from(e: NormalError) -> Self {
        MarketError::InvalidDistribution(e)
    }
}

#[derive(Clone)]
pub struct Market {
    config: MarketConfig,
    order_book: OrderBook,
    history: Vec<CandleData>,
}

#[derive(Clone, Copy)]
pub struct MarketConfig {
    pub market_size: usize,
    pub initial_open: f32,
    pub open_std: f32,
    pub skew: f32,
    pub n_runs: usize,
    pub min_quantity: f32,
    pub max_quantity: f32,
    pub buyer_ratio_std: f32,
}

impl MarketConfig {
    pub fn new(
        market_size: usize,
        initial_open: f32,
        open_std: f32,
        skew: f32,
        n_runs: usize,
        min_quantity: f32,
        max_quantity: f32,
        buyer_ratio_std: f32,
    ) -> MarketConfig {
        MarketConfig {
            market_size,
            initial_open,
            open_std,
            skew,
            n_runs,
            min_quantity,
            max_quantity,
            buyer_ratio_std,
        }
    }
}

impl Market {
    pub fn with_config(cfg: MarketConfig) -> Market {
        Market {
            config: cfg,
            order_book: OrderBook::default(),
            history: Vec::with_capacity(16),
        }
    }

    pub fn run(&mut self) -> Result<(), MarketError> {
        let cfg = self.config;
        let mut open = cfg.initial_open;
        let price_factor_dist: Normal<f32> = Normal::new(cfg.skew, cfg.open_std)?;
        let buyer_ratio_dist: Normal<f32> = Normal::new(0.5, cfg.buyer_ratio_std)?;
        for _ in 0..cfg.n_runs {
            open = self.run_single(open, &price_factor_dist, &buyer_ratio_dist)?;
        }
        Ok(())
    }

    fn run_single(
        &mut self,
        open: f32,
        price_factor_dist: &Normal<f32>,
        buyer_ratio_dist: &Normal<f32>,
    ) -> Result<f32, MarketError> {
        let cfg = self.config;
        let mut rng: rand::prelude::ThreadRng = rng();
        let buyer_ratio: f32 = buyer_ratio_dist.sample(&mut rng).clamp(0.4, 0.6);
        let n_buyers = (cfg.market_size as f32 * buyer_ratio).round() as usize;
        let n_sellers = cfg.market_size - n_buyers;

        let buy_orders: Vec<Order> = (0..n_buyers)
            .map(|_| {
                let factor: f32 = price_factor_dist.sample(&mut rng);
                Order::new(
                    open + open * factor,
                    rng.random_range(cfg.min_quantity..=cfg.max_quantity),
                )
            })
            .collect();
        let sell_orders: Vec<Order> = (0..n_sellers)
            .map(|_| {
                let factor: f32 = price_factor_dist.sample(&mut rng);
                Order::new(
                    open + open * factor,
                    rng.random_range(cfg.min_quantity..=cfg.max_quantity),
                )
            })
            .collect();
        self.order_book.insert_bids(buy_orders);
        self.order_book.insert_asks(sell_orders);
        let candle = self.order_book.resolve()?;
        self.history.push(candle);
        Ok(candle.close)
    }

    pub fn history_to_df(&self) -> Result<DataFrame, PolarsError> {
        let min: Vec<f32> = self.history.iter().map(|c| c.min).collect();
        let max: Vec<f32> = self.history.iter().map(|c| c.max).collect();
        let mean: Vec<f32> = self.history.iter().map(|c| c.mean).collect();
        let median: Vec<f32> = self.history.iter().map(|c| c.median).collect();
        let perc_25: Vec<f32> = self.history.iter().map(|c| c.perc_25).collect();
        let perc_75: Vec<f32> = self.history.iter().map(|c| c.perc_75).collect();
        let open: Vec<f32> = self.history.iter().map(|c| c.open).collect();
        let close: Vec<f32> = self.history.iter().map(|c| c.close).collect();

        let df = df!(
            "min" => min,
            "max" => max,
            "mean" => mean,
            "median" => median,
            "perc_25" => perc_25,
            "perc_75" => perc_75,
            "open" => open,
            "close" => close,
        )?;
        Ok(df)
    }

    pub fn history_to_parquet<W: Write>(&self, writer: W) -> Result<(), PolarsError> {
        let mut df = self.history_to_df()?;
        ParquetWriter::new(writer).finish(&mut df)?;
        Ok(())
    }

    pub fn history_to_csv<W: Write>(&self, writer: W) -> Result<(), PolarsError> {
        let mut df = self.history_to_df()?;
        CsvWriter::new(writer).finish(&mut df)?;
        Ok(())
    }
}
