use crate::order_book::{CandleData, Order, OrderBook};

#[derive(Clone)]
struct Market {
    size: usize,
    order_book: OrderBook,
    history: Vec<CandleData>,
}

impl Market {
    pub fn from_book(size: usize, order_book: OrderBook) -> Market {
        Market {
            size,
            order_book,
            history: Vec::with_capacity(16),
        }
    }

    pub fn from_orders(size: usize, bids: Vec<Order>, asks: Vec<Order>) -> Market {
        Market::from_book(size, OrderBook::new(bids, asks))
    }

    pub fn with_size(size: usize) -> Market {
        Market::from_book(size, OrderBook::default())
    }
}
