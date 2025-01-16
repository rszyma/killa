use crate::collector::colv2::some_worker;
use bottom::event::BottomEvent;
use collector::init::init_collector;
use iced::widget::{button, checkbox, column, container, responsive, scrollable, text};
use iced::{Element, Font, Length, Pixels, Renderer, Task, Theme};
use iced_table::table;
use std::fmt;
use std::sync::mpsc::Receiver;
use std::time::Instant;
mod collector;

fn main() {
    // Set collections thread as early as possible, to speed up first access to data.
    let collector_rx = init_collector();
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .default_font(Font {
            weight: iced::font::Weight::Medium,
            ..Default::default()
        })
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
            theme: Theme::Light,
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
                // dbg!(&data.list_of_batteries);
                let rows: &mut Vec<Row> = self.rows.as_mut();
                *rows = data
                    .list_of_processes
                    .unwrap_or_default()
                    .iter()
                    .map(|ps| Row {
                        program_name: ps.name.clone(),
                        mem: ps.mem_usage_bytes / 1_000_000,
                        cpu_perc: (((ps.cpu_usage_percent) / (num_cpus::get() as f32) * 10.0)
                            as i32) as f32
                            / 10.0,
                        pid: ps.pid,
                        command: ps.command.clone(),
                    })
                    .collect();
                rows.sort_by_key(|row| (100 * 10000) - (row.cpu_perc * 10000.0f32) as u32);
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
        // println!("view()");
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
        let font_size = Pixels::from(13.0);
        let content: Element<_> = match self.kind {
            ColumnKind::Name => text!("{}", row.program_name).size(font_size).into(),
            ColumnKind::Memory => text!("{} MB", row.mem).size(font_size).into(),
            ColumnKind::Cpu => text!("{} %", row.cpu_perc).size(font_size).into(),
            ColumnKind::Pid => text!("{}", row.pid).size(font_size).into(),
            ColumnKind::Command => text!("{}", row.command).size(font_size).into(),
            ColumnKind::Started => text!("{:?}", Instant::now()).size(font_size).into(),

            ColumnKind::Index => text(row_index).size(font_size).into(),
            ColumnKind::Delete => button(text("Delete").size(font_size))
                .on_press(Message::DeleteRow(row_index))
                .into(),
        };

        container(content).width(Length::Fill).center_y(15).into()
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn resize_offset(&self) -> Option<f32> {
        self.resize_offset
    }
}
