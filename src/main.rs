use crate::collector::colv2::run_collector_worker;
use crate::ui::{ColumnKind, Message, Row, TextInputAction};
use collector::init::init_collector;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::container::background;
use iced::widget::{
    self, checkbox, column, container, responsive, row, scrollable, text, text_input,
};
use iced::window::{self};
use iced::{
    Color, Element, Font, Length, Renderer, Size, Subscription, Task, Theme, border, color,
};
use process_data::{KillaData, ProcessListSort, SortOrder};
use rustix::process::{Signal, kill_process};

mod collect_uptimes;
mod collector;
mod keybinds;
mod process_data;
mod ui;

fn main() {
    iced::application(App::boot, App::update, App::view)
        .default_font(Font {
            weight: iced::font::Weight::Medium,
            ..Default::default()
        })
        .window(window::Settings {
            platform_specific: window::settings::PlatformSpecific {
                application_id: "killa".to_string(), // class
                ..Default::default()
            },
            size: Size::new(1024.0, 512.0),
            // icon: todo!(),
            ..Default::default()
        })
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
        .unwrap()
}

const SEARCH_INPUT_ID: iced::widget::Id = iced::widget::Id::new("global-search");

#[derive(Debug)]
pub struct SearchState {
    pub is_hidden: bool,
    pub text: String,
}

#[derive(Clone)]
pub enum FreezeState {
    Disabled,
    Enabled(Option<KillaData>), // holds latest collected data
}

struct App {
    pub columns: Vec<ColumnKind>,
    pub rows: Vec<Row>,
    pub table_top_id: widget::Id,
    pub theme: Theme,
    pub search: SearchState,
    pub sort: ProcessListSort,
    pub last_data: KillaData,
    pub freeze: FreezeState,
    pub staged_sig_all_filtered: Option<rustix::process::Signal>,
    pub wireframe_enabled: bool,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let collector_rx = init_collector();
        let init_collector_task =
            Task::stream(run_collector_worker(collector_rx)).then(|ev| match ev {
                collector::colv2::Event::Ready(_sender) => Task::none(),
                collector::colv2::Event::DataReady(data) => {
                    Task::done(Message::CollectedData(data))
                }
                collector::colv2::Event::WorkFinished => Task::none(),
            });
        (Self::default(), Task::batch(vec![init_collector_task]))
    }

    fn subscription(_: &Self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(Message::SystemThemeChanged),
            keybinds::handle_keybinds(),
        ])
    }

    fn theme(&self) -> Option<Theme> {
        Some(self.theme.clone())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Search(ev) => {
                self.staged_sig_all_filtered = None;
                return self.handle_search(ev);
            }
            Message::CollectedData(data) => {
                let kd = KillaData::from(data);
                if let FreezeState::Enabled(d) = &mut self.freeze {
                    d.replace(kd);
                    return Task::none();
                }
                self.last_data = kd;
                self.sort_rows();
                self.filter_rows();
            }
            Message::Freeze(enable) => {
                self.staged_sig_all_filtered = None;
                self.set_freeze(false);
                self.sort_rows();
                self.filter_rows();
                self.set_freeze(enable);
            }
            Message::ToggleFreeze => {
                self.staged_sig_all_filtered = None;
                if matches!(self.freeze, FreezeState::Enabled(_)) {
                    self.set_freeze(false);
                    self.sort_rows();
                    self.filter_rows();
                } else {
                    self.set_freeze(true)
                }
            }
            Message::ToggleWireframe(enabled) => self.wireframe_enabled = enabled,
            Message::SetSortField(sort_field) => {
                self.staged_sig_all_filtered = None;
                self.sort.column = sort_field;
                self.sort_rows();
                self.filter_rows();
            }
            Message::Back => {
                if self.staged_sig_all_filtered.is_some() {
                    self.staged_sig_all_filtered = None;
                }

                if matches!(self.freeze, FreezeState::Enabled(_)) {
                    self.set_freeze(false);
                    self.sort_rows();
                    self.filter_rows();
                } else {
                    return self.handle_search(TextInputAction::Hide);
                }
            }
            Message::Enter => {
                if let Some(sig) = self.staged_sig_all_filtered {
                    self.staged_sig_all_filtered = None;
                    self.sig_all_filtered(sig);
                    self.set_freeze(false);
                }
            }
            Message::StageSignalAllFiltered(sig) => {
                if matches!(self.freeze, FreezeState::Enabled(_)) && self.search.text.len() >= 3 {
                    self.staged_sig_all_filtered = Some(sig);
                }
            }
            Message::SystemThemeChanged(mode) => {
                self.theme = match mode {
                    iced::theme::Mode::Light => Theme::GruvboxLight,
                    iced::theme::Mode::Dark => Theme::GruvboxDark,
                    iced::theme::Mode::None => Theme::GruvboxDark,
                };
            }
        }

        Task::none()
    }

    /// This is called every time a Message has been processed in [`Self::update`]
    fn view(&self) -> iced::Element<'_, Message, Theme, Renderer> {
        let table = scrollable(responsive(|size| {
            // TODO: Make columns resizable
            let table: widget::table::Table<'_, Message> = widget::table::table(
                self.columns
                    .iter()
                    .map(|col_kind| {
                        widget::table::column(
                            widget::text(col_kind.to_string())
                                .width(size.width * col_kind.width_ratio()),
                            |row: Row| row.cell(col_kind),
                        )
                    })
                    .collect::<Vec<_>>(),
                self.rows.clone(),
            );
            table.into()
        }))
        .id(self.table_top_id.clone());

        let topbar_left = column![
            checkbox(matches!(self.freeze, FreezeState::Enabled(_))) // TODO: "Freeze" label
                .on_toggle(|_| Message::ToggleFreeze),
            checkbox(self.wireframe_enabled).on_toggle(Message::ToggleWireframe), // TODO: "Wireframe" label.
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

        if self.search.is_hidden {
            let search_box = text_input("Search processes", &self.search.text)
                .on_input(|text| Message::Search(TextInputAction::Replace(text)))
                .id(SEARCH_INPUT_ID);
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

        // fix transparency issues with table.
        let table = container(table).style(|theme| background(theme.palette().background));

        // red border on kill confirmation or cool blue on freeze.
        let table = if let Some(sig) = self.staged_sig_all_filtered {
            let color = match sig {
                Signal::Term => color!(0xFF0000), // red
                Signal::Kill => color!(0x9B26B6), // violet
                _ => color!(0xCCFF00),            // yellow
            };
            container(table).style(move |_theme| {
                container::bordered_box(&self.theme).border(border::width(10).color(color))
            })
        } else if matches!(self.freeze, FreezeState::Enabled(_)) {
            container(table).style(|_theme| {
                container::bordered_box(&self.theme)
                    .border(border::width(10).color(color!(0x6495ED)))
            })
        } else {
            container(table)
        }
        .padding(2);

        let content = column![topbar, table,].spacing(6);

        let all: Element<_> =
            container(container(content).width(Length::Fill).height(Length::Fill))
                .padding(20)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();

        if self.wireframe_enabled {
            all.explain(Color::from_rgb8(255, 0, 0))
        } else {
            all
        }
    }

    pub fn filter_rows(&mut self) {
        self.rows.clear();
        let a = self.last_data.clone();
        let a = match &self.search.is_hidden {
            true => a.search(&self.search.text),
            false => a,
        };
        let a: Vec<Row> = a.into();
        self.rows.extend(a);
    }

    pub fn sort_rows(&mut self) {
        let ProcessListSort { column, order } = self.sort;
        self.last_data.sort_by_column(column, order);
    }

    pub fn set_freeze(&mut self, enable: bool) {
        match (self.freeze.clone(), enable) {
            (FreezeState::Disabled, true) => {
                self.freeze = FreezeState::Enabled(None);
            }
            (FreezeState::Disabled, false) => {} // already disabled
            (FreezeState::Enabled(_), true) => {} // already enabled
            (FreezeState::Enabled(latest_collected_data), false) => {
                if let Some(data) = latest_collected_data {
                    self.last_data = data;
                };
                self.freeze = FreezeState::Disabled;
            }
        }
    }

    pub fn handle_search(&mut self, ev: TextInputAction) -> Task<Message> {
        let scroll_to_top = widget::operation::scroll_to(
            self.table_top_id.clone(),
            scrollable::AbsoluteOffset { x: 0., y: 0. },
        );

        let scroll_to_top_and_focus = Task::batch(vec![
            widget::operation::scroll_to(
                self.table_top_id.clone(),
                scrollable::AbsoluteOffset { x: 0., y: 0. },
            ),
            iced::widget::operation::focus(SEARCH_INPUT_ID),
        ]);

        match ev {
            TextInputAction::Append(s) => {
                if self.search.is_hidden {
                    self.search.text.push_str(&s);
                    self.set_freeze(false);
                } else {
                    self.search.is_hidden = true;
                    self.search.text = s;
                    self.set_freeze(false);
                }
                self.filter_rows();
                scroll_to_top_and_focus
            }
            TextInputAction::Replace(s) => {
                self.search.is_hidden = true;
                self.search.text = s;
                self.set_freeze(false);
                self.filter_rows();
                scroll_to_top_and_focus
            }
            TextInputAction::PopLastChar => {
                if self.search.is_hidden {
                    self.search.text.pop();
                    self.set_freeze(false);
                    self.filter_rows();
                    scroll_to_top_and_focus
                } else {
                    Task::none()
                }
            }
            TextInputAction::Toggle => {
                self.set_freeze(false);
                if self.search.is_hidden {
                    self.search.is_hidden = false;
                    self.filter_rows();
                    scroll_to_top_and_focus
                } else {
                    self.search.is_hidden = true;
                    self.filter_rows();
                    scroll_to_top_and_focus
                        .chain(iced::widget::operation::select_all(SEARCH_INPUT_ID))
                }
            }
            TextInputAction::Hide => {
                self.search.is_hidden = false;
                self.set_freeze(false);
                self.sort_rows();
                self.filter_rows();
                scroll_to_top
            }
        }
    }

    pub fn sig_all_filtered(&mut self, sig: rustix::process::Signal) {
        let pids: Vec<i32> = self.rows.iter().map(|x| x.pid).collect();
        for pid in pids {
            print!("sending {sig:?} to {pid} ... ");

            let result = kill_process(
                rustix::thread::Pid::from_raw(pid).expect("pid in table should be valid"),
                sig,
            );
            match result {
                Ok(()) => {
                    println!("ok");
                }
                Err(err) => {
                    println!("error: {err}");
                }
            }

            // refresh/refreeze
            self.set_freeze(false);
            self.sort_rows();
            self.filter_rows();
            self.set_freeze(true);
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            columns: vec![
                ColumnKind::Name,
                ColumnKind::Memory,
                ColumnKind::Cpu,
                ColumnKind::Pid,
                // ColumnKind::CpuTime, // todo
                // ColumnKind::Started, // todo
                ColumnKind::Command,
            ],
            rows: vec![],
            table_top_id: widget::Id::unique(),
            theme: Theme::Dark, // whatever startup theme, will be changed shortly.
            search: SearchState {
                is_hidden: false,
                text: String::new(),
            },
            sort: ProcessListSort {
                column: ColumnKind::Cpu,
                order: SortOrder::default(),
            },
            last_data: KillaData::default(),
            freeze: FreezeState::Disabled,
            wireframe_enabled: false,
            staged_sig_all_filtered: None,
        }
    }
}

// impl<'a> iced::Program for App<'a> {
//     type State = State<'a>;
//     type Message = Message;
//     type Theme = Theme;
//     type Renderer = iced::Renderer;
//     type Executor = iced::executor::Default;

//     fn name() -> &'static str {
//         "killa"
//     }

//     fn theme(&self, state: &Self::State, _window: iced::window::Id) -> Option<Self::Theme> {
//         Some(state.theme.clone())
//     }

//     fn settings(&self) -> iced::Settings {
//         todo!()
//     }

//     fn window(&self) -> Option<iced::window::Settings> {
//         todo!()
//     }

//     fn boot(&self) -> (Self::State, Task<Self::Message>) {
//         todo!()
//     }

// }
