use crate::collector::colv2::some_worker;
use bottom::event::BottomEvent;
use collector::init::init_collector;
use iced::widget::{
    self, button, checkbox, column, container, responsive, row, scrollable, text, text_input, Space,
};
use iced::{
    event, keyboard, Element, Event, Font, Length, Pixels, Renderer, Subscription, Task, Theme,
};
use iced_core::widget::operation;
use iced_table::table::{self};
use process_data::{KillaData, ProcessListSort, SortOrder};
use std::fmt;
use std::sync::mpsc::Receiver;
use std::time::Instant;

mod collector;
mod keyboard_thingy;
mod process_data;

fn main() {
    // Set collections thread as early as possible, to speed up first access to data.
    let collector_rx = init_collector();
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .default_font(Font {
            weight: iced::font::Weight::Medium,
            ..Default::default()
        })
        .subscription(App::subscription)
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
        // let subscribe_to_keyboard = Task::stream(keyboard_event_producer()).then(|ev| match ev {
        //     collector::colv2::Event::Ready(_sender) => Task::none(),
        //     collector::colv2::Event::DataReady(data) => Task::done(Message::CollectedData(data)),
        //     collector::colv2::Event::WorkFinished => Task::none(),
        // });
        (
            App::default(),
            Task::batch(vec![
                init_collector_task,
                // subscribe_to_keyboard,
            ]),
        )
    }
}

#[derive(Debug, Clone)]
enum TextInputAction {
    Append(String),
    Backspace,
    Replace(String),
    Toggle,
    Hide,
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
    Search(TextInputAction),
}

struct App {
    columns: Vec<Column>,
    rows: Vec<Row>,
    header: scrollable::Id,
    body: scrollable::Id,
    resize_columns_enabled: bool,
    theme: Theme,
    search: Option<String>,
    sort: ProcessListSort,
    last_data: KillaData,
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
                // Column::new(ColumnKind::Started),
            ],
            rows: vec![],
            header: scrollable::Id::unique(),
            body: scrollable::Id::unique(),
            resize_columns_enabled: true,
            theme: Theme::Light,
            search: None,
            sort: ProcessListSort {
                column: ColumnKind::Cpu,
                order: SortOrder::default(),
            },
            last_data: KillaData::default(),
        }
    }
}

const SEARCH_INPUT_ID: &str = "global-search";

impl App {
    fn title(&self) -> String {
        "killa".into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _window| -> Option<Message> {
            use keyboard::key::Named as K;
            use keyboard::Key as T;

            if let event::Status::Captured = status {
                // Shortcuts in search box active
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                    match key {
                        T::Character(c) => {
                            if modifiers.control() && (c == "f") {
                                return Some(Message::Search(TextInputAction::Hide));
                            }
                        }
                        T::Named(K::Escape) => {
                            return Some(Message::Search(TextInputAction::Hide));
                        }
                        _ => {}
                    }
                };
                return None;
            };

            // Out-of-search-box keyboard shortcuts.
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                match key {
                    T::Named(K::Backspace) => {
                        return Some(Message::Search(TextInputAction::Backspace))
                    }
                    T::Character(c) => {
                        if modifiers.control() && (c == "f") {
                            return Some(Message::Search(TextInputAction::Toggle));
                        } else {
                            return Some(Message::Search(TextInputAction::Append(c.to_string())));
                        };
                    }
                    _ => {}
                }
            };

            None
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Search(ev) => {
                match ev {
                    TextInputAction::Append(s) => match &mut self.search {
                        Some(text) => text.push_str(&s),
                        None => {
                            self.search.replace(s);
                        }
                    },
                    TextInputAction::Replace(s) => {
                        let _ = self.search.insert(s);
                    }
                    TextInputAction::Backspace => {
                        if let Some(x) = &mut self.search {
                            x.pop();
                        }
                    }
                    TextInputAction::Toggle => match &self.search {
                        Some(_) => {
                            self.filter_rows();
                            self.search.take();
                        }
                        None => {
                            let _ = self.search.insert("".into());
                            return text_input::focus(SEARCH_INPUT_ID)
                                .chain(text_input::select_all(SEARCH_INPUT_ID));
                        }
                    },
                    TextInputAction::Hide => {
                        self.search.take();
                        self.filter_rows();
                        return Task::none();
                    }
                };
                self.filter_rows();
                return text_input::focus(SEARCH_INPUT_ID);
            }
            Message::CollectedData(data) => {
                let kd = KillaData::from(data);
                self.last_data = kd;
                self.sort_rows();
                self.filter_rows();
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
            let mut table = iced_table::table(
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

        let topbar_left = column![
            checkbox("Resize Columns", self.resize_columns_enabled,)
                .on_toggle(Message::ResizeColumnsEnabled),
            checkbox("Dark Theme", matches!(self.theme, Theme::Dark),)
                .on_toggle(Message::DarkThemeEnabled),
        ]
        .spacing(6);

        let mut topbar_items: Vec<iced_core::Element<Message, _, _>> = vec![topbar_left.into()];

        if let Some(search_text) = &self.search {
            // let no_widget = Space::new(0, 0);
            let search_box = text_input("Search processes", search_text)
                .on_input(|text| Message::Search(TextInputAction::Replace(text)))
                .id(text_input::Id::new(SEARCH_INPUT_ID));
            topbar_items.push(search_box.into());
        };

        let topbar = iced::widget::Row::from_iter(topbar_items).spacing(6);

        let content = column![topbar, table].spacing(6);

        container(container(content).width(Length::Fill).height(Length::Fill))
            .padding(20)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn filter_rows(&mut self) {
        self.rows.clear();
        let a = self.last_data.clone();
        let a = match &self.search {
            Some(phrase) => a.search(phrase),
            None => a,
        };
        let a: Vec<Row> = a.into();
        self.rows.extend(a);
    }

    fn sort_rows(&mut self) {
        let ProcessListSort { column, order } = self.sort;
        self.last_data.sort_by_column(column, order);
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

#[derive(Clone, Copy)]
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
#[derive(Clone)]
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
