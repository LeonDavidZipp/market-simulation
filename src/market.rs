use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, PartialEq)]
struct Order {
    price: f32,
    quantity: f32,
    is_buy: bool,
}

impl Order {
    pub fn new_buy(price: f32, quantity: f32) -> Order {
        Order {
            price,
            quantity,
            is_buy: true,
        }
    }

    pub fn new_sell(price: f32, quantity: f32) -> Order {
        Order {
            price,
            quantity,
            is_buy: false,
        }
    }
}

struct OrderBook {
    bids: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
    asks: BTreeMap<OrderedFloat<f32>, VecDeque<Order>>,
}

impl OrderBook {
    pub fn new(orders: Vec<Order>) -> OrderBook {
        let mut book = OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
        for order in orders {
            book.insert(order);
        }
        book
    }

    pub fn empty() -> OrderBook {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    fn insert(&mut self, order: Order) {
        let side = if order.is_buy {
            &mut self.bids
        } else {
            &mut self.asks
        };
        side.entry(OrderedFloat(order.price))
            .or_insert_with(VecDeque::new)
            .push_back(order);
    }

    pub fn resolve(&mut self) {
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
        let orders = sample_orders();
        let mut book = OrderBook::new(orders);

        assert_eq!(book.bids.len(), 3);
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        let (low_price, mut low_level) = book.bids.pop_first().unwrap();

        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_back(), Some(Order::new_buy(12.0, 13.0)));
        assert_eq!(low_price, 11.0);
        assert_eq!(low_level.pop_front(), Some(Order::new_buy(11.0, 12.0)));

        assert_eq!(book.asks.len(), 3);
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        let (low_price, mut low_level) = book.asks.pop_first().unwrap();

        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_back(), Some(Order::new_sell(12.0, 13.0)));
        assert_eq!(low_price, 11.0);
        assert_eq!(low_level.pop_front(), Some(Order::new_sell(11.0, 12.0)));
    }

    #[test]
    fn test_resolve_simple() {
        let orders = simple_resolveable_orders();
        let mut book = OrderBook::new(orders);
        book.resolve();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_resolve_leftover_buy() {
        let orders = leftover_buy();
        let mut book = OrderBook::new(orders);
        book.resolve();

        assert!(!book.bids.is_empty());
        let (top_price, mut top_level) = book.bids.pop_last().unwrap();
        assert_eq!(top_price, 11.0);
        assert_eq!(top_level.pop_front(), Some(Order::new_buy(11.0, 12.0)));
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_resolve_leftover_sell() {
        let orders = leftover_sell();
        let mut book = OrderBook::new(orders);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (top_price, mut top_level) = book.asks.pop_last().unwrap();
        assert_eq!(top_price, 12.0);
        assert_eq!(top_level.pop_front(), Some(Order::new_sell(12.0, 12.0)));
    }

    #[test]
    fn test_partially_resolve_full() {
        let orders = partially_resolve_full();
        let mut book = OrderBook::new(orders);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_partially_resolve_leftover_buy() {
        let orders = partially_resolve_leftover_buy();
        let mut book = OrderBook::new(orders);
        book.resolve();

        assert!(book.asks.is_empty());
        assert!(!book.bids.is_empty());
        let (price, mut level) = book.bids.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new_buy(11.5, 1.0)));
        assert!(level.is_empty());
    }

    #[test]
    fn test_partially_resolve_leftover_sell() {
        let orders = partially_resolve_leftover_sell();
        let mut book = OrderBook::new(orders);
        book.resolve();

        assert!(book.bids.is_empty());
        assert!(!book.asks.is_empty());
        let (price, mut level) = book.asks.pop_last().unwrap();
        assert_eq!(price, 11.5);
        assert_eq!(level.pop_front(), Some(Order::new_sell(11.5, 1.0)));
        assert!(level.is_empty());
    }

    fn sample_orders() -> Vec<Order> {
        vec![
            Order::new_buy(11.0, 12.0),
            Order::new_buy(12.0, 13.0),
            Order::new_buy(11.5, 130.0),
            Order::new_sell(11.0, 12.0),
            Order::new_sell(12.0, 13.0),
            Order::new_sell(11.5, 130.0),
        ]
    }

    fn simple_resolveable_orders() -> Vec<Order> {
        vec![Order::new_buy(11.0, 12.0), Order::new_sell(11.0, 12.0)]
    }

    fn leftover_buy() -> Vec<Order> {
        vec![
            Order::new_buy(11.0, 12.0),
            Order::new_buy(12.0, 12.0),
            Order::new_sell(11.5, 12.0),
        ]
    }

    fn leftover_sell() -> Vec<Order> {
        vec![
            Order::new_buy(11.5, 12.0),
            Order::new_sell(11.0, 12.0),
            Order::new_sell(12.0, 12.0),
        ]
    }

    fn partially_resolve_full() -> Vec<Order> {
        vec![
            Order::new_buy(11.5, 25.0),
            Order::new_sell(11.0, 12.0),
            Order::new_sell(11.5, 13.0),
        ]
    }

    fn partially_resolve_leftover_buy() -> Vec<Order> {
        vec![
            Order::new_buy(11.5, 26.0),
            Order::new_sell(11.0, 12.0),
            Order::new_sell(11.5, 13.0),
        ]
    }

    fn partially_resolve_leftover_sell() -> Vec<Order> {
        vec![
            Order::new_buy(11.5, 24.0),
            Order::new_sell(11.0, 12.0),
            Order::new_sell(11.5, 13.0),
        ]
    }
}
