use bottom::event::BottomEvent;
use collector::init::init_collector;
use core::num;
use iced::widget::{
    button, checkbox, column, container, pick_list, responsive, scrollable, text, text_input,
};
use iced::{Element, Length, Renderer, Subscription, Task, Theme};
use iced_table::table;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

use crate::collector::colv2::some_worker;

mod collector;

fn main() {
    // Set collections thread as early as possible, to speed up first access to data.
    let collector_rx = init_collector();
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .run_with(init(collector_rx))
        .unwrap()
}

fn init(collector_rx: Receiver<BottomEvent>) -> impl FnOnce() -> (App, iced::Task<Message>) {
    || {
        let init_collector_task = Task::stream(some_worker(collector_rx)).then(|ev| match ev {
            collector::colv2::Event::Ready(_sender) => Task::none(),
            collector::colv2::Event::DataReady(data) => Task::done(Message::CollectedData(data)),
            collector::colv2::Event::WorkFinished => Task::none(),
        });
        (App::default(), init_collector_task)
    }
}

/// Messages that update UI.
#[derive(Debug, Clone)]
enum Message {
    CollectedData(Box<bottom::data_collection::Data>),
    SyncHeader(scrollable::AbsoluteOffset),
    Resizing(usize, f32),
    Resized,
    ResizeColumnsEnabled(bool),
    DarkThemeEnabled(bool),
    DeleteRow(usize),
}

struct App {
    columns: Vec<Column>,
    rows: Vec<Row>,
    header: scrollable::Id,
    body: scrollable::Id,
    resize_columns_enabled: bool,
    min_width_enabled: bool,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            columns: vec![
                Column::new(ColumnKind::Name),
                Column::new(ColumnKind::Memory),
                Column::new(ColumnKind::Cpu),
                Column::new(ColumnKind::Pid),
                Column::new(ColumnKind::Command),
                Column::new(ColumnKind::Started),
            ],
            rows: vec![],
            header: scrollable::Id::unique(),
            body: scrollable::Id::unique(),
            resize_columns_enabled: true,
            min_width_enabled: true,
            theme: Theme::Dark,
        }
    }
}

impl App {
    fn title(&self) -> String {
        "killa".into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     // I've no idea how to control frequency of subscriptions calls...
    //     Subscription::none()
    // }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CollectedData(data) => {
                dbg!("tock");

                // todo: collect this only once
                let num_of_cpu = num_cpus::get();
                dbg!(num_of_cpu);
                assert!(num_of_cpu > 0);

                // dbg!(&data.list_of_batteries);
                let rows: &mut Vec<Row> = self.rows.as_mut();
                *rows = data
                    .list_of_processes
                    .unwrap_or_default()
                    .iter()
                    .map(|ps| Row {
                        program_name: ps.name.clone(),
                        mem: ps.mem_usage_bytes / 1_000_000,
                        cpu_perc: (((ps.cpu_usage_percent) / (num_of_cpu as f32) * 100.0) as i32)
                            as f32
                            / 100.0,
                        pid: ps.pid,
                        command: ps.command.clone(),
                    })
                    .collect();
                rows.sort_by_key(|row| (100 * 100) - (row.cpu_perc as u32 * 100));
            }
            Message::SyncHeader(offset) => {
                return Task::batch(vec![
                    scrollable::scroll_to(self.header.clone(), offset),
                    // scrollable::scroll_to(self.footer.clone(), offset),
                ]);
            }
            Message::Resizing(index, offset) => {
                if let Some(column) = self.columns.get_mut(index) {
                    column.resize_offset = Some(offset);
                }
            }
            Message::Resized => self.columns.iter_mut().for_each(|column| {
                if let Some(offset) = column.resize_offset.take() {
                    column.width += offset;
                }
            }),
            Message::ResizeColumnsEnabled(enabled) => self.resize_columns_enabled = enabled,
            Message::DarkThemeEnabled(enabled) => {
                if enabled {
                    self.theme = Theme::Dark;
                } else {
                    self.theme = Theme::Light;
                }
            }
            Message::DeleteRow(index) => {
                self.rows.remove(index);
            }
        }

        Task::none()
    }

    /// This is called every time a Message has been processed in [`Self::update`]
    fn view(&self) -> Element<Message> {
        println!("view()");
        let table = responsive(|size| {
            let mut table = table(
                self.header.clone(),
                self.body.clone(),
                &self.columns,
                &self.rows,
                Message::SyncHeader,
            );

            if self.resize_columns_enabled {
                table = table.on_column_resize(Message::Resizing, Message::Resized);
            }
            table = table.min_width(size.width); // this autoresizes table width to window
            table.into()
        });

        let content = column![
            checkbox("Resize Columns", self.resize_columns_enabled,)
                .on_toggle(Message::ResizeColumnsEnabled),
            checkbox("Dark Theme", matches!(self.theme, Theme::Dark),)
                .on_toggle(Message::DarkThemeEnabled),
            table,
        ]
        .spacing(6);

        container(container(content).width(Length::Fill).height(Length::Fill))
            .padding(20)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

struct Column {
    kind: ColumnKind,
    width: f32,
    resize_offset: Option<f32>,
}

impl Column {
    fn new(kind: ColumnKind) -> Self {
        let width = match kind {
            ColumnKind::Name => 350.0,
            ColumnKind::Memory => 100.0,
            ColumnKind::Cpu => 60.0,
            ColumnKind::Pid => 80.0,
            ColumnKind::Command => 400.0,
            ColumnKind::Started => 150.0,

            ColumnKind::Index => 10.0,  // 60.0,
            ColumnKind::Delete => 10.0, // 100.0,
        };

        Self {
            kind,
            width,
            resize_offset: None,
        }
    }
}

// name, memory, CPU, pid, command line, started

enum ColumnKind {
    Name,
    Memory,
    Cpu,
    Pid,
    Command,
    Started,

    Index,
    Delete,
}

/// Actual storage for data row.
struct Row {
    program_name: String,
    mem: u64,
    cpu_perc: f32,
    pid: i32,
    command: String,
    // notes: String,
    // category: Category,
    // is_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    A,
    B,
    C,
    D,
    E,
}

impl Category {
    const ALL: &'static [Self] = &[Self::A, Self::B, Self::C, Self::D, Self::E];
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::A => "A",
            Category::B => "B",
            Category::C => "C",
            Category::D => "D",
            Category::E => "E",
        }
        .fmt(f)
    }
}

impl<'a> table::Column<'a, Message, Theme, Renderer> for Column {
    type Row = Row;

    fn header(&'a self, _col_index: usize) -> Element<'a, Message> {
        let content = match self.kind {
            ColumnKind::Name => "Name",
            ColumnKind::Memory => "Memory",
            ColumnKind::Cpu => "CPU",
            ColumnKind::Pid => "ID",
            ColumnKind::Command => "Command",
            ColumnKind::Started => "Started",

            ColumnKind::Index => "Index",
            ColumnKind::Delete => "",
        };

        container(text(content)).center_y(24).into()
    }

    fn cell(&'a self, _col_index: usize, row_index: usize, row: &'a Row) -> Element<'a, Message> {
        let content: Element<_> = match self.kind {
            ColumnKind::Name => text!("{}", row.program_name).into(),
            ColumnKind::Memory => text!("{} MB", row.mem).into(),
            ColumnKind::Cpu => text!("{} %", row.cpu_perc).into(),
            ColumnKind::Pid => text!("{}", row.pid).into(),
            ColumnKind::Command => text!("{}", row.command).into(),
            ColumnKind::Started => text!("{:?}", Instant::now()).into(),

            ColumnKind::Index => text(row_index).into(),
            ColumnKind::Delete => button(text("Delete"))
                .on_press(Message::DeleteRow(row_index))
                .into(),
        };

        container(content).width(Length::Fill).center_y(32).into()
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn resize_offset(&self) -> Option<f32> {
        self.resize_offset
    }
}
