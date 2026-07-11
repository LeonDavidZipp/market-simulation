use crate::math::{calc_25th_percentile, calc_75th_percentile, calc_median};
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::{self, Display};

#[derive(Clone)]
pub struct OrderBook {
    pub last_traded_price: f32,
    pub bids: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
    pub asks: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
}

impl Default for OrderBook {
    fn default() -> OrderBook {
        OrderBook {
            last_traded_price: f32::INFINITY,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
}

impl OrderBook {
    /// Inserts `order` into the bid or ask side of the book, depending on
    /// `is_bid`.
    pub fn insert_order(&mut self, order: Order, is_bid: bool) {
        let side = if is_bid {
            &mut self.bids
        } else {
            &mut self.asks
        };
        side.entry(OrderedFloat(order.price))
            .or_default()
            .push_back(order);
    }

    /// Matches crossing bids and asks, filling orders until the best bid no
    /// longer meets or exceeds the best ask.
    ///
    /// Returns the sequence of trade prices produced by this call, or `None`
    /// if no trade occurred.
    pub fn resolve(&mut self, inserted_is_bid: bool) -> Option<Vec<f32>> {
        let bids = &mut self.bids;
        let asks = &mut self.asks;
        let mut trade_prices: Option<Vec<f32>> = None;
        loop {
            let Some((&bid_price, _)) = bids.iter().next_back() else {
                break;
            };
            let Some((&ask_price, _)) = asks.iter().next() else {
                break;
            };
            if bid_price < ask_price {
                break;
            }
            let Some(bid_orders) = bids.get_mut(&bid_price) else {
                break;
            };
            let Some(bid_order) = bid_orders.back_mut() else {
                break;
            };
            let Some(ask_orders) = asks.get_mut(&ask_price) else {
                break;
            };
            let Some(ask_order) = ask_orders.front_mut() else {
                break;
            };
            let bid_quant = &mut bid_order.quantity;
            let ask_quant = &mut ask_order.quantity;
            let filled = (*bid_quant).min(*ask_quant);
            *bid_quant -= filled;
            *ask_quant -= filled;
            if *bid_quant <= 0.0 {
                bid_orders.pop_back();
                if bid_orders.is_empty() {
                    bids.remove(&bid_price);
                }
            }
            if *ask_quant <= 0.0 {
                ask_orders.pop_front();
                if ask_orders.is_empty() {
                    asks.remove(&ask_price);
                }
            }
            if inserted_is_bid {
                trade_prices
                    .get_or_insert_with(Vec::new)
                    .push(ask_price.into_inner());
            } else {
                trade_prices
                    .get_or_insert_with(Vec::new)
                    .push(bid_price.into_inner());
            }
        }
        trade_prices
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Order {
    pub price: f32,
    pub quantity: f32,
}

impl Order {
    /// Creates a new order for `quantity` units at `price`.
    pub fn new(price: f32, quantity: f32) -> Order {
        Order { price, quantity }
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.quantity, self.price)
    }
}

#[derive(Clone, Copy)]
pub struct CandleData {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub median: f32,
    pub perc_25: f32,
    pub perc_75: f32,
    pub open: f32,
    pub close: f32,
}

impl CandleData {
    /// Aggregates a slice of trade prices into a single [`CandleData`]
    /// (min/max/mean/median/25th/75th percentile/open/close).
    ///
    /// # Errors
    ///
    /// Returns [`EmptyDataError`] if `data` is empty.
    pub fn from_data(data: &[f32]) -> Result<CandleData, EmptyDataError> {
        if data.is_empty() {
            return Err(EmptyDataError);
        }
        let mut d_copy = data.to_vec();
        d_copy.sort_unstable_by(f32::total_cmp);
        let (min, max, sum) = data
            .iter()
            .copied()
            .fold((f32::MAX, f32::MIN, 0.0), |(min, max, sum), x| {
                (min.min(x), max.max(x), sum + x)
            });
        let mean = sum / data.len() as f32;
        let median = calc_median(&d_copy).ok_or(EmptyDataError)?;
        let perc_25 = calc_25th_percentile(&d_copy).ok_or(EmptyDataError)?;
        let perc_75 = calc_75th_percentile(&d_copy).ok_or(EmptyDataError)?;
        let open = *data.first().ok_or(EmptyDataError)?;
        let close = *data.last().ok_or(EmptyDataError)?;
        Ok(CandleData {
            min,
            max,
            mean,
            median,
            perc_25,
            perc_75,
            open,
            close,
        })
    }
}

#[derive(Debug)]
pub struct EmptyDataError;

impl Display for EmptyDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data is empty")
    }
}

impl std::error::Error for EmptyDataError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_data_from_data_empty() {
        let result = CandleData::from_data(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_candle_data_from_data_single_value() {
        let candle = CandleData::from_data(&[11.0]).unwrap();
        assert_eq!(candle.min, 11.0);
        assert_eq!(candle.max, 11.0);
        assert_eq!(candle.mean, 11.0);
        assert_eq!(candle.median, 11.0);
        assert_eq!(candle.perc_25, 11.0);
        assert_eq!(candle.perc_75, 11.0);
        assert_eq!(candle.open, 11.0);
        assert_eq!(candle.close, 11.0);
    }

    #[test]
    fn test_candle_data_from_data_multiple_values() {
        let data: [f32; 5] = [0.1, 1.2, 0.2, 0.03, 0.04];
        let candle = CandleData::from_data(&data).unwrap();

        assert_eq!(candle.min, 0.03);
        assert_eq!(candle.max, 1.2);
        assert_eq!(candle.mean, 0.314);
        assert_eq!(candle.median, 0.1);
        assert_eq!(candle.perc_25, 0.04);
        assert_eq!(candle.perc_75, 0.2);
        assert_eq!(candle.open, 0.1);
        assert_eq!(candle.close, 0.04);
    }

    #[test]
    fn test_default_is_empty() {
        let book = OrderBook::default();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert_eq!(book.last_traded_price, f32::INFINITY);
    }

    #[test]
    fn test_insert_bid() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.0, 12.0), true);

        assert_eq!(book.bids.len(), 1);
        let level = book.bids.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 1);
        assert_eq!(level.front(), Some(&Order::new(11.0, 12.0)));
    }

    #[test]
    fn test_insert_ask() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.0, 12.0), false);

        assert_eq!(book.asks.len(), 1);
        let level = book.asks.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 1);
        assert_eq!(level.front(), Some(&Order::new(11.0, 12.0)));
    }

    #[test]
    fn test_resolve_simple() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.0, 12.0), true);
        book.insert_order(Order::new(11.0, 12.0), false);
        let trades = book.resolve(false);

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert_eq!(trades, Some(vec![11.0]));
    }

    #[test]
    fn test_resolve_leftover_buy() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.0, 12.0), true);
        book.insert_order(Order::new(12.0, 12.0), true);
        book.insert_order(Order::new(11.5, 12.0), false);
        let trades = book.resolve(false);
        println!("{:?}, {:?}", book.bids, book.asks);

        assert!(!book.bids.is_empty());
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        assert_eq!(top_price, 11.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(11.0, 12.0)));
        assert!(book.asks.is_empty());

        assert_eq!(trades, Some(vec![12.0]));
    }

    #[test]
    fn test_resolve_leftover_sell() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.5, 12.0), true);
        book.insert_order(Order::new(11.0, 12.0), false);
        book.insert_order(Order::new(12.0, 12.0), false);
        let trades = book.resolve(false);

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(12.0, 12.0)));

        assert_eq!(trades, Some(vec![11.5]));
    }

    #[test]
    fn test_partially_resolve_full() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.5, 25.0), true);
        book.insert_order(Order::new(11.0, 12.0), false);
        book.insert_order(Order::new(11.5, 13.0), false);
        let trades = book.resolve(false);

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert_eq!(trades, Some(vec![11.5, 11.5]));
    }

    #[test]
    fn test_partially_resolve_leftover_buy() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.5, 26.0), true);
        book.insert_order(Order::new(11.0, 12.0), false);
        book.insert_order(Order::new(11.5, 13.0), false);
        let trades = book.resolve(false);

        assert!(book.asks.is_empty());
        assert!(!book.bids.is_empty());
        let (price, mut level) = book.bids.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());

        assert_eq!(trades, Some(vec![11.5, 11.5]));
    }

    #[test]
    fn test_partially_resolve_leftover_sell() {
        let mut book = OrderBook::default();
        book.insert_order(Order::new(11.5, 24.0), true);
        book.insert_order(Order::new(11.0, 12.0), false);
        book.insert_order(Order::new(11.5, 13.0), false);
        let trades = book.resolve(false);

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (price, mut level) = book.asks.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());

        assert_eq!(trades, Some(vec![11.5, 11.5]));
    }
}
