use bottom::event::BottomEvent;
use iced::futures::Stream;
use iced_futures::subscription::EventStream;
use iced_futures::subscription::Hasher;
use iced_futures::subscription::Recipe;
use iced_futures::BoxStream;
use std::cell::Cell;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Duration, Instant};
use std::{default, fmt, thread};

use bottom::app::DataFilters;
use bottom::create_collection_thread;
use bottom::data_collection::temperature::TemperatureType;
use bottom::data_collection::DataCollector;
use iced::futures::{self, StreamExt};
use iced::widget::{
    button, checkbox, column, container, pick_list, responsive, scrollable, text, text_input,
};
use iced::{Element, Event, Length, Renderer, Subscription, Task, Theme};
use iced_futures::subscription::{self, from_recipe};
use iced_futures::{boxed_stream, MaybeSend};
use iced_table::table;

#[derive(Clone)]
pub(crate) struct Collector {
    pub(crate) rx: Arc<Mutex<mpsc::Receiver<BottomEvent>>>, // todo: get rid of Arc
}

impl Collector {
    pub(crate) fn new(rx: Arc<Mutex<mpsc::Receiver<BottomEvent>>>) -> Self {
        Self { rx }
    }
}

impl Stream for Collector {
    type Item = Box<bottom::data_collection::Data>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // todo: how often is it polled?
        dbg!("polled!");
        let lock = self.rx.lock().unwrap();
        match lock.try_recv() {
            Ok(BottomEvent::Update(x)) => {
                dbg!("have some updates!!");
                Poll::Ready(Some(x))
            }
            Ok(_) => Poll::Pending,
            Err(err) => {
                match err {
                    mpsc::TryRecvError::Empty => Poll::Pending,
                    mpsc::TryRecvError::Disconnected => {
                        // panic!("disconnected from btm collector")
                        // todo: log some error or something?
                        Poll::Ready(None)
                    }
                }
            }
        }
    }
}

impl Recipe for Collector {
    type Output = Box<bottom::data_collection::Data>;

    fn hash(&self, state: &mut Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _: EventStream) -> BoxStream<Self::Output> {
        self.clone().boxed()
    }
}
