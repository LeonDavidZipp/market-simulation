use crate::order_book::EmptyDataError;
use crate::order_book::{CandleData, Order, OrderBook};
use polars::error::PolarsError;
use polars::prelude::{CsvWriter, DataFrame, ParquetWriter, SerWriter, df};
use rand::rng;
use rand_distr::uniform::Error as UniformError;
use rand_distr::{Binomial, BinomialError, Distribution, Normal, NormalError, Uniform};
use std::io::Write;

#[derive(Debug)]
pub enum MarketError {
    EmptyData(EmptyDataError),
    InvalidDistribution(NormalError),
    InvalidBinomial(BinomialError),
    InvalidUniform(UniformError),
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketError::EmptyData(e) => write!(f, "empty data: {e}"),
            MarketError::InvalidDistribution(e) => write!(f, "invalid distribution: {e}"),
            MarketError::InvalidBinomial(e) => write!(f, "invalid binomial: {e}"),
            MarketError::InvalidUniform(e) => write!(f, "invalid uniform: {e}"),
        }
    }
}

impl std::error::Error for MarketError {}

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

impl From<BinomialError> for MarketError {
    fn from(e: BinomialError) -> Self {
        MarketError::InvalidBinomial(e)
    }
}

impl From<UniformError> for MarketError {
    fn from(e: UniformError) -> Self {
        MarketError::InvalidUniform(e)
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
    pub n_traders: usize,
    pub trade_prob: f32,
    pub initial_open: f32,
    pub order_price_std: f32,
    pub skew: f32,
    pub n_steps: usize,
    pub n_ticks_per_candle: usize,
    pub min_quantity: f32,
    pub max_quantity: f32,
    pub shock_prob: f32,
    pub shock_intensity: f32,
    pub shock_intensity_std: f32,
    pub spike_ratio: f32,
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
        let book = &mut self.order_book;
        book.last_traded_price = cfg.initial_open;
        let mut rng: rand::prelude::ThreadRng = rng();
        let price_factor_dist: Normal<f32> = Normal::new(cfg.skew, cfg.order_price_std)?;
        let orders_dist = Binomial::new(cfg.n_traders as u64, cfg.trade_prob as f64)?;
        let quantity_dist = Uniform::new_inclusive(cfg.min_quantity, cfg.max_quantity)?;
        let n_ticks_total = cfg.n_steps * cfg.n_ticks_per_candle;
        let mut tick = 1;
        let mut trade_prices: Vec<f32> = Vec::with_capacity(cfg.n_ticks_per_candle);
        let shock_prob_dist: Uniform<f32> = Uniform::new_inclusive(0.0, 1.0)?;
        let shock_intensity_dist: Normal<f32> =
            Normal::new(cfg.shock_intensity, cfg.shock_intensity_std)?;
        let shock_type_dist: Uniform<f32> = Uniform::new_inclusive(0.0, 1.0)?;
        for _ in 0..n_ticks_total {
            let n_orders: u64 = orders_dist.sample(&mut rng);
            for _ in 0..n_orders {
                let price = book.last_traded_price * (1.0 + price_factor_dist.sample(&mut rng));
                let quantity = quantity_dist.sample(&mut rng);
                let order = Order::new(price, quantity);
                let inserted_is_bid = price >= book.last_traded_price;
                if inserted_is_bid {
                    book.insert_bid(order);
                } else {
                    book.insert_ask(order);
                }
                let Some(mut data) = book.resolve(inserted_is_bid) else {
                    continue;
                };
                trade_prices.append(&mut data);
                if let Some(&last) = trade_prices.last() {
                    book.last_traded_price = last;
                }
            }
            if tick == cfg.n_ticks_per_candle {
                let candle = CandleData::from_data(&trade_prices);
                if let Ok(c) = candle {
                    self.history.push(c);
                }
                tick = 0;
                trade_prices.clear();

                if shock_prob_dist.sample(&mut rng) >= cfg.shock_prob {
                    let intensity = shock_intensity_dist.sample(&mut rng);
                    if shock_type_dist.sample(&mut rng) < cfg.spike_ratio {
                        book.last_traded_price *= 1.0 - intensity;
                    } else {
                        book.last_traded_price *= 1.0 + intensity;
                    }
                }
            } else {
                tick += 1;
            };
        }
        Ok(())
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
