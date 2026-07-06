use crate::math::{calc_25th_percentile, calc_75th_percentile, calc_median};
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Order {
    pub price: f32,
    pub quantity: f32,
}

impl Order {
    pub fn new(price: f32, quantity: f32) -> Order {
        Order { price, quantity }
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
    pub fn new(
        min: f32,
        max: f32,
        mean: f32,
        median: f32,
        perc_25: f32,
        perc_75: f32,
        open: f32,
        close: f32,
    ) -> CandleData {
        CandleData {
            min,
            max,
            mean,
            median,
            perc_25,
            perc_75,
            open,
            close,
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
        let open = *data.first().ok_or(EmptyDataError)?;
        let close = *data.last().ok_or(EmptyDataError)?;
        Ok(CandleData::new(
            min, max, mean, median, perc_25, perc_75, open, close,
        ))
    }
}

#[derive(Debug)]
pub struct EmptyDataError;

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
    pub fn insert_bid(&mut self, order: Order) {
        self.bids
            .entry(OrderedFloat(order.price))
            .or_insert_with(VecDeque::new)
            .push_back(order);
    }

    pub fn insert_bids(&mut self, orders: Vec<Order>) {
        let mut grouped: HashMap<OrderedFloat<f32>, VecDeque<Order>> = HashMap::new();
        for order in orders {
            grouped
                .entry(OrderedFloat(order.price))
                .or_default()
                .push_back(order);
        }
        for (price, mut queue) in grouped {
            self.bids
                .entry(price)
                .or_insert_with(VecDeque::new)
                .append(&mut queue);
        }
    }

    pub fn insert_ask(&mut self, order: Order) {
        self.asks
            .entry(OrderedFloat(order.price))
            .or_insert_with(VecDeque::new)
            .push_back(order);
    }

    pub fn insert_asks(&mut self, orders: Vec<Order>) {
        let mut grouped: HashMap<OrderedFloat<f32>, VecDeque<Order>> = HashMap::new();
        for order in orders {
            grouped
                .entry(OrderedFloat(order.price))
                .or_default()
                .push_back(order);
        }
        for (price, mut queue) in grouped {
            self.asks
                .entry(price)
                .or_insert_with(VecDeque::new)
                .append(&mut queue);
        }
    }

    pub fn resolve(&mut self) -> Result<Vec<f32>, EmptyDataError> {
        let mut data: Vec<f32> = Vec::with_capacity(self.bids.len());
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
            data.push(ask_price.into_inner());
        }
        Ok(data)
    }
}

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
        book.insert_bid(Order::new(11.0, 12.0));

        assert_eq!(book.bids.len(), 1);
        let level = book.bids.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 1);
        assert_eq!(level.front(), Some(&Order::new(11.0, 12.0)));
    }

    #[test]
    fn test_insert_bids_groups_by_price_and_preserves_order() {
        let mut book = OrderBook::default();
        book.insert_bids(vec![
            Order::new(11.0, 1.0),
            Order::new(12.0, 2.0),
            Order::new(11.0, 3.0),
        ]);

        assert_eq!(book.bids.len(), 2);
        let level_11 = book.bids.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level_11.len(), 2);
        assert_eq!(level_11.front(), Some(&Order::new(11.0, 1.0)));
        assert_eq!(level_11.back(), Some(&Order::new(11.0, 3.0)));

        let level_12 = book.bids.get(&OrderedFloat(12.0)).unwrap();
        assert_eq!(level_12.len(), 1);
        assert_eq!(level_12.front(), Some(&Order::new(12.0, 2.0)));
    }

    #[test]
    fn test_insert_bids_appends_to_existing_level() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.0, 1.0));
        book.insert_bids(vec![Order::new(11.0, 2.0)]);

        let level = book.bids.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 2);
        assert_eq!(level.front(), Some(&Order::new(11.0, 1.0)));
        assert_eq!(level.back(), Some(&Order::new(11.0, 2.0)));
    }

    #[test]
    fn test_insert_ask() {
        let mut book = OrderBook::default();
        book.insert_ask(Order::new(11.0, 12.0));

        assert_eq!(book.asks.len(), 1);
        let level = book.asks.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 1);
        assert_eq!(level.front(), Some(&Order::new(11.0, 12.0)));
    }

    #[test]
    fn test_insert_asks_groups_by_price_and_preserves_order() {
        let mut book = OrderBook::default();
        book.insert_asks(vec![
            Order::new(11.0, 1.0),
            Order::new(12.0, 2.0),
            Order::new(11.0, 3.0),
        ]);

        assert_eq!(book.asks.len(), 2);
        let level_11 = book.asks.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level_11.len(), 2);
        assert_eq!(level_11.front(), Some(&Order::new(11.0, 1.0)));
        assert_eq!(level_11.back(), Some(&Order::new(11.0, 3.0)));

        let level_12 = book.asks.get(&OrderedFloat(12.0)).unwrap();
        assert_eq!(level_12.len(), 1);
        assert_eq!(level_12.front(), Some(&Order::new(12.0, 2.0)));
    }

    #[test]
    fn test_insert_asks_appends_to_existing_level() {
        let mut book = OrderBook::default();
        book.insert_ask(Order::new(11.0, 1.0));
        book.insert_asks(vec![Order::new(11.0, 2.0)]);

        let level = book.asks.get(&OrderedFloat(11.0)).unwrap();
        assert_eq!(level.len(), 2);
        assert_eq!(level.front(), Some(&Order::new(11.0, 1.0)));
        assert_eq!(level.back(), Some(&Order::new(11.0, 2.0)));
    }

    #[test]
    fn test_resolve_simple() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.0, 12.0));
        book.insert_ask(Order::new(11.0, 12.0));
        let trades = book.resolve().unwrap();

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert_eq!(trades, vec![11.0]);
    }

    #[test]
    fn test_resolve_leftover_buy() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.0, 12.0));
        book.insert_bid(Order::new(12.0, 12.0));
        book.insert_ask(Order::new(11.5, 12.0));
        let trades = book.resolve().unwrap();

        assert!(!book.bids.is_empty());
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        assert_eq!(top_price, 11.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(11.0, 12.0)));
        assert!(book.asks.is_empty());

        assert_eq!(trades, vec![11.5]);
    }

    #[test]
    fn test_resolve_leftover_sell() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.5, 12.0));
        book.insert_ask(Order::new(11.0, 12.0));
        book.insert_ask(Order::new(12.0, 12.0));
        let trades = book.resolve().unwrap();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_front(), Some(Order::new(12.0, 12.0)));

        assert_eq!(trades, vec![11.0]);
    }

    #[test]
    fn test_partially_resolve_full() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.5, 25.0));
        book.insert_ask(Order::new(11.0, 12.0));
        book.insert_ask(Order::new(11.5, 13.0));
        let trades = book.resolve().unwrap();

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
        assert_eq!(trades, vec![11.0, 11.5]);
    }

    #[test]
    fn test_partially_resolve_leftover_buy() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.5, 26.0));
        book.insert_ask(Order::new(11.0, 12.0));
        book.insert_ask(Order::new(11.5, 13.0));
        let trades = book.resolve().unwrap();

        assert!(book.asks.is_empty());
        assert!(!book.bids.is_empty());
        let (price, mut level) = book.bids.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());

        assert_eq!(trades, vec![11.0, 11.5]);
    }

    #[test]
    fn test_partially_resolve_leftover_sell() {
        let mut book = OrderBook::default();
        book.insert_bid(Order::new(11.5, 24.0));
        book.insert_ask(Order::new(11.0, 12.0));
        book.insert_ask(Order::new(11.5, 13.0));
        let trades = book.resolve().unwrap();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (price, mut level) = book.asks.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new(11.5, 1.0)));
        assert!(level.is_empty());

        assert_eq!(trades, vec![11.0, 11.5]);
    }
}
