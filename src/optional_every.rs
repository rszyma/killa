//! Listen and react to time.
use iced::futures;

use crate::subscription::{self, Hasher, Subscription};

/// Returns a [`Subscription`] that produces messages at a set interval.
///
/// The first message is produced after a `duration`, and then continues to
/// produce more messages every `duration` after that.
pub fn every(duration: std::time::Duration) -> Subscription<std::time::Instant> {
    subscription::from_recipe(Every(duration))
}

#[derive(Debug)]
struct Every(std::time::Duration);

impl subscription::Recipe for Every {
    type Output = std::time::Instant;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.0.hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: subscription::EventStream,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        use futures::stream::StreamExt;

        let start = tokio::time::Instant::now() + self.0;

        let mut interval = tokio::time::interval_at(start, self.0);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let stream = {
            futures::stream::unfold(interval, |mut interval| async move {
                Some((interval.tick().await, interval))
            })
        };

        stream.map(tokio::time::Instant::into_std).boxed()
    }
}
