use std::cell::Cell;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::task::Poll;
use std::time::{Duration, Instant};
use std::{default, fmt, thread};

use bottom::app::DataFilters;
use bottom::create_collection_thread;
use bottom::data_collection::temperature::TemperatureType;
use bottom::data_collection::DataCollector;
use bottom::event::BottomEvent;
use iced::futures::channel::mpsc as iced_mpsc;
use iced::futures::{self, Stream, StreamExt};
use iced::widget::{
    button, checkbox, column, container, pick_list, responsive, scrollable, text, text_input,
};
use iced::{Element, Event, Length, Renderer, Subscription, Task, Theme};
use iced_futures::subscription::{self, from_recipe, EventStream, Hasher, Recipe};
use iced_futures::{boxed_stream, BoxStream, MaybeSend};
use iced_table::table;

// We have custom filter_map since the one in iced is not public.
pub fn filter_map<I, F, T>(id: I, f: F) -> Subscription<T>
where
    I: core::hash::Hash + 'static,
    F: Fn(iced_futures::subscription::Event) -> Option<T> + MaybeSend + 'static,
    T: 'static + MaybeSend,
{
    from_recipe(Runner {
        id,
        spawn: |events| {
            use futures::future;
            use futures::stream::StreamExt;

            events.filter_map(move |event| future::ready(f(event)))
        },
    })
}

struct Runner<I, F, S, T>
where
    F: FnOnce(EventStream) -> S,
    S: Stream<Item = T>,
{
    id: I,
    spawn: F,
}

impl<I, F, S, T> Recipe for Runner<I, F, S, T>
where
    I: core::hash::Hash + 'static,
    F: FnOnce(EventStream) -> S,
    S: Stream<Item = T> + MaybeSend + 'static,
{
    type Output = T;

    fn hash(&self, state: &mut Hasher) {
        std::any::TypeId::of::<I>().hash(state);
        self.id.hash(state);
    }

    fn stream(self: Box<Self>, input: EventStream) -> BoxStream<Self::Output> {
        boxed_stream((self.spawn)(input))
    }
}
