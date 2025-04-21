use crate::collector::colv2::run_collector_worker;
use bottom::event::BottomEvent;
use collector::init::init_collector;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::tooltip::Position;
use iced::widget::{
    button, checkbox, column, container, responsive, row, scrollable, text, text_input, tooltip,
};
use iced::window::{self};
use iced::{
    event, keyboard, Color, Element, Event, Font, Length, Pixels, Renderer, Size, Subscription,
    Task, Theme,
};
use iced_table::table::{self};
use process_data::{KillaData, ProcessListSort, SortOrder};
use std::sync::mpsc::Receiver;
use std::time::Duration;

mod collect_uptimes;
mod collector;
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
        .window(window::Settings {
            size: Size::new(1024.0, 512.0),
            // icon: todo!(),
            ..Default::default()
        })
        .subscription(App::subscription)
        .run_with(init(collector_rx))
        .unwrap()
}

fn init(collector_rx: Receiver<BottomEvent>) -> impl FnOnce() -> (App, iced::Task<Message>) {
    || {
        let init_collector_task =
            Task::stream(run_collector_worker(collector_rx)).then(|ev| match ev {
                collector::colv2::Event::Ready(_sender) => Task::none(),
                collector::colv2::Event::DataReady(data) => {
                    Task::done(Message::CollectedData(data))
                }
                collector::colv2::Event::WorkFinished => Task::none(),
            });
        (App::default(), Task::batch(vec![init_collector_task]))
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
    ToggleFreeze,
    ToggleWireframe(bool),
    SetSortField(ColumnKind),
    DeleteRow(usize),
    Search(TextInputAction),
}

struct App {
    columns: Vec<Column>,
    rows: Vec<Row>,
    header: scrollable::Id,
    body: scrollable::Id,
    theme: Theme,
    search: Option<String>,
    sort: ProcessListSort,
    last_data: KillaData,
    freeze: bool,
    enable_wireframe: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            columns: vec![
                Column::new(ColumnKind::Name),
                Column::new(ColumnKind::Memory),
                Column::new(ColumnKind::CPU),
                Column::new(ColumnKind::PID),
                // Column::new(ColumnKind::CpuTime),
                // Column::new(ColumnKind::Started), // todo
                Column::new(ColumnKind::Command),
            ],
            rows: vec![],
            header: scrollable::Id::unique(),
            body: scrollable::Id::unique(),
            theme: Theme::Dark,
            search: None,
            sort: ProcessListSort {
                column: ColumnKind::CPU,
                order: SortOrder::default(),
            },
            last_data: KillaData::default(),
            freeze: false,
            enable_wireframe: false,
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
            use iced::keyboard::Modifiers as M;
            use keyboard::key::Named as K;
            use keyboard::Key as T;

            // Right now, there's only one widget that can capture input - seach box.
            let is_search_box_active = matches!(status, event::Status::Captured);

            let (key, modifiers) = if let Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) = event
            {
                (key, modifiers)
            } else {
                return None;
            };

            const NO_MODS: M = M::empty();

            // keybinds
            let res: Option<_> = match (modifiers, key.as_ref(), is_search_box_active) {
                ////////////////////////
                // No modifiers
                (NO_MODS, T::Character(c), false) => {
                    Some(Message::Search(TextInputAction::Append(c.to_string())))
                }
                (NO_MODS, T::Named(K::Backspace), _) => {
                    Some(Message::Search(TextInputAction::Backspace))
                }
                (NO_MODS, T::Named(K::Escape), true) => {
                    Some(Message::Search(TextInputAction::Hide))
                }

                ////////////////////////
                // CTRL modifier
                (M::CTRL, T::Character("f"), true) => Some(Message::Search(TextInputAction::Hide)),
                (M::CTRL, T::Character("f"), false) => {
                    Some(Message::Search(TextInputAction::Toggle))
                }
                (M::CTRL, T::Character("d"), _) => Some(Message::ToggleFreeze),
                (M::CTRL, T::Character("1"), _) => Some(Message::SetSortField(ColumnKind::CPU)),
                (M::CTRL, T::Character("2"), _) => Some(Message::SetSortField(ColumnKind::Memory)),
                (M::CTRL, T::Character("3"), _) => Some(Message::SetSortField(ColumnKind::PID)),

                _ => None,
            };

            res
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

                let tasks: Vec<Task<Message>> = vec![
                    scrollable::scroll_to(
                        self.body.clone(),
                        scrollable::AbsoluteOffset { x: 0., y: 0. },
                    ),
                    text_input::focus(SEARCH_INPUT_ID),
                ];
                return Task::batch(tasks);
            }
            Message::CollectedData(data) => {
                if self.freeze {
                    return Task::none();
                }
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
            Message::ToggleFreeze => self.freeze = !self.freeze,
            Message::ToggleWireframe(enabled) => self.enable_wireframe = enabled,
            Message::SetSortField(sort_field) => {
                self.sort.column = sort_field;
                self.sort_rows();
                self.filter_rows();
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

            // Make columns resizable.
            table = table.on_column_resize(Message::Resizing, Message::Resized);

            table = table.min_width(size.width); // this autoresizes table width to window
            table.into()
        });

        let topbar_left = column![
            checkbox("Freeze ️❄️", self.freeze).on_toggle(|_| Message::ToggleFreeze),
            checkbox("Wireframe", self.enable_wireframe).on_toggle(Message::ToggleWireframe),
            text(format!("Sorting By {:?}", self.sort.column))
        ]
        .spacing(6);

        let total_memory_usage = {
            let used = (self.last_data.memory.used_bytes as f64) / 1_000_000_000.0;
            let total = (self.last_data.memory.total_bytes as f64) / 1_000_000_000.0;
            let percent = self.last_data.memory.checked_percent().unwrap_or_default();
            iced::widget::text(format!(
                "Memory: {:.1} GB ({:.1}%) of {:.0} GB",
                used, percent, total,
            ))
        };

        let mut topbar = row![topbar_left].spacing(6);

        if let Some(search_text) = &self.search {
            let search_box = text_input("Search processes", search_text)
                .on_input(|text| Message::Search(TextInputAction::Replace(text)))
                .id(text_input::Id::new(SEARCH_INPUT_ID));
            topbar = topbar.push(container(search_box).padding(10).align_y(Vertical::Center));
        } else {
            topbar = topbar.push(container("").width(Length::Fill));
        }

        // iced::widget::vertical_rule(1).into()
        topbar = topbar.push(
            container(
                total_memory_usage
                    .align_y(Vertical::Center)
                    // .align_x(Horizontal::Center)
                    .width(Length::Shrink)
                    .height(Length::Fill),
            )
            .height(Length::Fixed(50.0))
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .padding(10),
        );

        let content = column![topbar, table].spacing(6);

        let content: Element<_> = container(
            content,
            // .align_x(Horizontal::Center)
            // .width(Length::Shrink)
            // .height(Length::Fill),
        )
        .width(Length::Fill)
        .padding(5)
        .into();

        let all: Element<_> =
            container(container(content).width(Length::Fill).height(Length::Fill))
                .padding(20)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();

        if self.enable_wireframe {
            all.explain(Color::from_rgb8(255, 0, 0))
        } else {
            all
        }
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
            ColumnKind::CPU => 60.0,
            ColumnKind::PID => 80.0,
            ColumnKind::Command => 400.0,
            ColumnKind::CpuTime => 100.0,
            ColumnKind::Started => 100.0,

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

#[derive(Clone, Copy, Debug)]
pub enum ColumnKind {
    Name,
    Memory,
    CPU,
    PID,
    Command,
    Started,
    CpuTime,

    Index,
    Delete,
}

/// Actual storage for data row.
#[derive(Clone)]
struct Row {
    program_name: String,
    program_name_lowercase: String, // index for search
    mem: u64,
    cpu_perc: f32,
    pid: i32,
    command: String,
    command_lowercase: String, // index for search
    cpu_time: Duration,
    // start_time: Option<chrono::DateTime<chrono::Local>>,
}

impl<'a> table::Column<'a, Message, Theme, Renderer> for Column {
    type Row = Row;

    fn header(&'a self, _col_index: usize) -> Element<'a, Message> {
        let content = match self.kind {
            ColumnKind::Name => "Name",
            ColumnKind::Memory => "Memory",
            ColumnKind::CPU => "CPU",
            ColumnKind::PID => "ID",
            ColumnKind::CpuTime => "Time",

            ColumnKind::Started => "Started",

            ColumnKind::Command => "Command",

            ColumnKind::Index => "Index",
            ColumnKind::Delete => "",
        };

        container(text(content)).center_y(24).into()
    }

    fn cell(&'a self, _col_index: usize, row_index: usize, row: &'a Row) -> Element<'a, Message> {
        // let a = humantime::format_duration(val);

        let font_size = Pixels::from(13.0);
        let tooltip_font_size = Pixels::from(11.0);
        let content: Element<_> = match self.kind {
            ColumnKind::Name => text!("{}", row.program_name).size(font_size).into(),
            ColumnKind::Memory => text!("{} MB", row.mem).size(font_size).into(),
            ColumnKind::CPU => text!("{:.1} %", row.cpu_perc).size(font_size).into(),
            ColumnKind::PID => text!("{}", row.pid).size(font_size).into(),
            ColumnKind::Command => tooltip(
                text!("{}", row.command).size(font_size),
                container(text!("{}", row.command).size(tooltip_font_size))
                    .padding(10)
                    .style(|theme: &Theme| {
                        let palette = theme.extended_palette();
                        let color_set = palette.background.weak;
                        let alpha = 0.99;
                        iced::widget::container::Style {
                            background: Some(color_set.color.scale_alpha(alpha).into()),
                            text_color: Some(color_set.text.scale_alpha(alpha)),
                            border: iced::border::rounded(5)
                                .color(palette.secondary.weak.color)
                                .width(1.5),
                            ..Default::default()
                        }
                    })
                    .max_width(700),
                Position::Bottom,
            )
            .into(),
            ColumnKind::Started => text!("{}", {
                // TODO: optimize for startup times.
                // Current implementation is pretty detrimental with the Mutex, it probably adds like 100ms.
                // Maybe pre-fetch the process start times?
                // get_process_start_time(row.pid)
                //     .unwrap_or_default()
                //     .format("%d/%m/%Y %H:%M")
                "-"
            })
            .size(font_size)
            .into(),
            ColumnKind::CpuTime => text!("{}", humantime::format_duration(row.cpu_time))
                .size(font_size)
                .into(),

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
