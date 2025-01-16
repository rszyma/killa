use std::time::Duration;

use bottom::event::BottomEvent;
use iced_futures::futures::channel::mpsc;
use iced_futures::futures::sink::SinkExt;
use iced_futures::futures::Stream;
use iced_futures::stream;
use tokio::time::sleep;

use super::init::init_collector;

pub enum Event {
    Ready(mpsc::Sender<Input>),
    WorkFinished,
    DataReady(Box<bottom::data_collection::Data>),
    // ...
}

#[derive(Debug)]
pub enum Input {
    DoSomeWork,
    // ...
}

// currently this panics, idk
#[inline(never)]
pub fn some_worker() -> impl Stream<Item = Event> {
    stream::channel(100, |mut output| async move {
        let collector_rx = init_collector();

        // Create channel for communication back to app.
        let (sender, mut receiver) = mpsc::channel(100);

        // Send the sender back to the application
        output.send(Event::Ready(sender)).await;

        loop {
            // use iced_futures::futures::StreamExt;
            // let input = receiver.select_next_some().await;

            // print!("looping...");

            if let Ok(BottomEvent::Update(data)) = collector_rx.try_recv() {
                // println!("sending data!");
                output.send(Event::DataReady(data)).await;
                continue;
            }
            sleep(Duration::from_millis(25)).await;

            // let input = receiver.next().await;

            // dbg!(&input);

            // match input {
            //     Some(input) => {
            //         match input {
            //             Input::DoSomeWork => {
            //                 // Do some async work...

            //                 // Finally, we can optionally produce a message to tell the
            //                 // `Application` the work is done
            //                 output.send(Event::WorkFinished).await;
            //             }
            //         }
            //     }
            //     None => {
            //         sleep(Duration::from_secs(1)).await;
            //     }
            // }
        }
    })
}
