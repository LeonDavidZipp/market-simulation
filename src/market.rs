use crate::order_book::EmptyDataError;
use crate::order_book::{CandleData, Order, OrderBook};
use polars::error::PolarsError;
use polars::prelude::{DataFrame, df};
use rand::RngExt;
use rand::rng;
use rand_distr::{Distribution, Normal, NormalError};

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
struct Market {
    size: usize,
    order_book: OrderBook,
    history: Vec<CandleData>,
}

struct RunConfig {
    start_prize: f32,
    n_runs: usize,
    min_quantity: f32,
    max_quantity: f32,
    buyer_ratio_stddev: f32,
}

impl Market {
    pub fn with_size(size: usize) -> Market {
        Market {
            size,
            order_book: OrderBook::default(),
            history: Vec::with_capacity(16),
        }
    }

    pub fn run(&mut self, cfg: &RunConfig) -> Result<(), MarketError> {
        let mut start_prize = cfg.start_prize;
        let buyer_ratio_dist: Normal<f32> = Normal::new(0.5, cfg.buyer_ratio_stddev)?;
        let mut rng: rand::prelude::ThreadRng = rng();
        for _ in 0..cfg.n_runs {
            start_prize = self.run_single(cfg, start_prize, &mut rng, &buyer_ratio_dist)?;
        }
        Ok(())
    }

    fn run_single(
        &mut self,
        cfg: &RunConfig,
        start_prize: f32,
        rng: &mut rand::prelude::ThreadRng,
        buyer_ratio_dist: &Normal<f32>,
    ) -> Result<f32, MarketError> {
        let buyer_ratio: f32 = buyer_ratio_dist.sample(rng).clamp(0.4, 0.6);
        let n_buyers = (self.size as f32 * buyer_ratio).round() as usize;
        let n_sellers = self.size - n_buyers;

        let buy_orders: Vec<Order> = (0..n_buyers)
            .map(|_| {
                Order::new(
                    start_prize,
                    rng.random_range(cfg.min_quantity..=cfg.max_quantity),
                )
            })
            .collect();
        let sell_orders: Vec<Order> = (0..n_sellers)
            .map(|_| {
                Order::new(
                    start_prize,
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
}
