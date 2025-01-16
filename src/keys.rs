use iced::{
    futures::{channel::mpsc, FutureExt, SinkExt, StreamExt, TryFutureExt},
    Subscription,
};
use iced_native::subscription;

pub enum State {
    Starting,
    // Ready(mpsc::UnboundedReceiver<rdev::Event>),
}
#[derive(Debug, Clone)]
pub enum Event {
    Ready,
    // KeyRecieved(rdev::Event),
}

// #[derive(Debug, Clone)]
// pub struct Connection(mpsc::UnboundedSender<rdev::Event>);

// pub fn bind() -> Subscription<Event> {
//     struct Keys;

//     subscription::unfold(
//         std::any::TypeId::of::<Keys>(),
//         State::Starting,
//         |state| async move {
//             match state {
//                 State::Starting => {
//                     let (mut sender, receiver) = mpsc::unbounded();
//                     std::thread::spawn(move || {
//                         listen(move |event| {
//                             sender
//                                 .send(event)
//                                 .unwrap_or_else(|e| println!("Could not send event {:?}", e))
//                                 .now_or_never();
//                         })
//                         .expect("Could not listen");
//                     });
//                     (Some(Event::Ready), State::Ready(receiver))
//                 }
//                 State::Ready(mut input) => {
//                     let received = input.next().await;
//                     match received {
//                         Some(key) => (Some(Event::KeyRecieved(key)), State::Ready(input)),
//                         None => (None, State::Ready(input)),
//                     }
//                 }
//             }
//         },
//     )
// }
