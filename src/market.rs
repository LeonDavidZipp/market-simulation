use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, VecDeque};
use crate::math::{calc_median, calc_25th_percentile, calc_75th_percentile};

#[derive(Debug, PartialEq)]
struct Order {
    price: f32,
    quantity: f32,
}

impl Order {
    pub fn new(price: f32, quantity: f32) -> Order {
        Order { price, quantity }
    }
}

#[derive(Clone, Copy)]
struct CandleData {
    min: f32,
    max: f32,
    mean: f32,
    median: f32,
    perc_25: f32,
    perc_75: f32,
}

impl CandleData {
    pub fn new(
        min: f32,
        max: f32,
        mean: f32,
        median: f32,
        perc_25: f32,
        perc_75: f32,
    ) -> CandleData {
        CandleData {
            min,
            max,
            mean,
            median,
            perc_25,
            perc_75,
        }
    }

    pub fn from_data(data: &[f32]) -> Result<CandleData, EmptyDataError> {
        let max = data.iter().copied().fold(f32::MIN, f32::max);
        let min = data.iter().copied().fold(f32::MAX, f32::min);
        let sum: f32 = data.iter().copied().sum();
        let mean = sum / data.len() as f32;
        let median = calc_median(data).ok_or(EmptyDataError)?;
        let perc_25 = calc_25th_percentile(data).ok_or(EmptyDataError)?;
        let perc_75 = calc_75th_percentile(data).ok_or(EmptyDataError)?;
        Ok(CandleData::new(
            min, max, mean, median, perc_25, perc_75
        ))
    }
}

#[derive(Debug)]
pub struct EmptyDataError;

struct OrderBook {
    bids: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
    asks: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
}

impl OrderBook {
    pub fn new(bids: Vec<Order>, asks: Vec<Order>) -> OrderBook {
        let mut book = OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
        for b in bids {
            book.insert_bid(b);
        }
        for a in asks {
            book.insert_ask(a);
        }
        book
    }

    pub fn empty() -> OrderBook {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    fn insert_bid(&mut self, order: Order) {
        self.bids
            .entry(OrderedFloat(order.price))
            .or_insert_with(VecDeque::new)
            .push_back(order);
    }

    fn insert_ask(&mut self, order: Order) {
        self.asks
            .entry(OrderedFloat(order.price))
            .or_insert_with(VecDeque::new)
            .push_back(order);
    }

    pub fn resolve(&mut self) {
        let max_price: f32 = 0.0;
        let max_price: f32 = 0.0;
        let max_price: f32 = 0.0;
        let max_price: f32 = 0.0;
        loop {
            // get highest bid price
            let Some((&bid_price, _)) = self.bids.iter().next_back() else {
                break;
            };
            // get lowest ask price
            let Some((&ask_price, _)) = self.asks.iter().next() else {
                break;
            };
            // check if bid > ask
            if bid_price < ask_price {
                break;
            }
            // trade
            // get foremost bid order
            let Some(bid_orders) = self.bids.get_mut(&bid_price) else {
                break;
            };
            let Some(bid_order) = bid_orders.back_mut() else {
                break;
            };
            // get foremost ask order
            let Some(ask_orders) = self.asks.get_mut(&ask_price) else {
                break;
            };
            let Some(ask_order) = ask_orders.front_mut() else {
                break;
            };

            let filled = ask_order.quantity.min(bid_order.quantity);
            bid_order.quantity -= filled;
            ask_order.quantity -= filled;
            if bid_order.quantity <= 0.0 {
                bid_orders.pop_front();
            }
            if bid_orders.is_empty() {
                self.bids.remove(&bid_price);
            }
            if ask_order.quantity <= 0.0 {
                ask_orders.pop_back();
            }
            if ask_orders.is_empty() {
                self.asks.remove(&ask_price);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let (bids, asks) = sample_orders();
        let mut book = OrderBook::new(bids, asks);

        assert_eq!(book.bids.len(), 3);
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        let (low_price, mut low_level) = book.bids.pop_first().unwrap();

        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_back(), Some(Order::new(12.0, 13.0)));
        assert_eq!(low_price, 11.0);
        assert_eq!(low_level.pop_front(), Some(Order::new(11.0, 12.0)));

        assert_eq!(book.asks.len(), 3);
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        let (low_price, mut low_level) = book.asks.pop_first().unwrap();

        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_back(), Some(Order::new(12.0, 13.0)));
        assert_eq!(low_price, 11.0);
        assert_eq!(low_level.pop_front(), Some(Order::new(11.0, 12.0)));
    }

    #[test]
    fn test_resolve_simple() {
        let (bids, asks) = simple_resolveable_orders();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_resolve_leftover_buy() {
        let (bids, asks) = leftover_buy();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();

        assert!(!book.bids.is_empty());
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        assert_eq!(top_price, 11.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(11.0, 12.0)));
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_resolve_leftover_sell() {
        let (bids, asks) = leftover_sell();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(12.0, 12.0)));
    }

    #[test]
    fn test_partially_resolve_full() {
        let (bids, asks) = partially_resolve_full();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_partially_resolve_leftover_buy() {
        let (bids, asks) = partially_resolve_leftover_buy();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();

        assert!(book.asks.is_empty());
        assert!(!book.bids.is_empty());
        let (price, mut level) = book.bids.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());
    }

    #[test]
    fn test_partially_resolve_leftover_sell() {
        let (bids, asks) = partially_resolve_leftover_sell();
        let mut book = OrderBook::new(bids, asks);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (price, mut level) = book.asks.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());
    }

    fn sample_orders() -> (Vec<Order>, Vec<Order>) {
        (
            vec![
                Order::new(11.0, 12.0),
                Order::new(12.0, 13.0),
                Order::new(11.5, 130.0),
            ],
            vec![
                Order::new(11.0, 12.0),
                Order::new(12.0, 13.0),
                Order::new(11.5, 130.0),
            ],
        )
    }

    fn simple_resolveable_orders() -> (Vec<Order>, Vec<Order>) {
        (vec![Order::new(11.0, 12.0)], vec![Order::new(11.0, 12.0)])
    }

    fn leftover_buy() -> (Vec<Order>, Vec<Order>) {
        (
            vec![Order::new(11.0, 12.0), Order::new(12.0, 12.0)],
            vec![Order::new(11.5, 12.0)],
        )
    }

    fn leftover_sell() -> (Vec<Order>, Vec<Order>) {
        (
            vec![Order::new(11.5, 12.0)],
            vec![Order::new(11.0, 12.0), Order::new(12.0, 12.0)],
        )
    }

    fn partially_resolve_full() -> (Vec<Order>, Vec<Order>) {
        (
            vec![Order::new(11.5, 25.0)],
            vec![Order::new(11.0, 12.0), Order::new(11.5, 13.0)],
        )
    }

    fn partially_resolve_leftover_buy() -> (Vec<Order>, Vec<Order>) {
        (
            vec![Order::new(11.5, 26.0)],
            vec![Order::new(11.0, 12.0), Order::new(11.5, 13.0)],
        )
    }

    fn partially_resolve_leftover_sell() -> (Vec<Order>, Vec<Order>) {
        (
            vec![Order::new(11.5, 24.0)],
            vec![Order::new(11.0, 12.0), Order::new(11.5, 13.0)],
        )
    }
}
