use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;
use std::{default, fmt, thread};

use bottom::app::DataFilters;
use bottom::create_collection_thread;
use bottom::data_collection::temperature::TemperatureType;
use bottom::data_collection::DataCollector;
use bottom::event::BottomEvent;
use iced::widget::{
    button, checkbox, column, container, pick_list, responsive, scrollable, text, text_input,
};
use iced::{Element, Length, Renderer, Task, Theme};
use iced_table::table;

fn main() {
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .run()
        .unwrap()
}

#[derive(Debug, Clone)]
enum Message {
    SyncHeader(scrollable::AbsoluteOffset),
    Resizing(usize, f32),
    Resized,
    ResizeColumnsEnabled(bool),
    MinWidthEnabled(bool),
    DarkThemeEnabled(bool),
    EditNotes(usize, String),
    CategoryChange(usize, Category),
    EnabledRow(usize, bool),
    DeleteRow(usize),
}

struct Collector {
    pub last_data: Arc<Mutex<Box<bottom::data_collection::Data>>>,
}

impl Collector {
    fn new(rx: mpsc::Receiver<BottomEvent>) -> Self {
        let self_ = Self {
            last_data: Default::default(),
        };
        let last_data_clone = self_.last_data.clone();
        thread::spawn(move || {
            for e in rx.iter() {
                if let BottomEvent::Update(data) = e {
                    // dbg!(&data.collection_time);
                    for cpu_item in data.clone().cpu.unwrap_or_default() {
                        if let bottom::data_collection::cpu::CpuDataType::Cpu(n) =
                            cpu_item.data_type
                        {
                            println!("cpu: {} usage: {}", n, cpu_item.cpu_usage);
                        }
                    }
                    println!("=============");
                    let last_data = last_data_clone.clone();
                    let mut data_store = last_data.lock().unwrap();
                    *data_store = data; // TODO: doesn't this do bitwise copy?
                }
            }
        });
        self_
    }
}

struct App {
    columns: Vec<Column>,
    rows: Vec<Row>,
    header: scrollable::Id,
    body: scrollable::Id,
    resize_columns_enabled: bool,
    min_width_enabled: bool,
    theme: Theme,
    collector: Collector,
}

impl Default for App {
    fn default() -> Self {
        let cancellation_token =
            Arc::new(bottom::utils::cancellation_token::CancellationToken::default());

        let (tx, rx) = mpsc::channel(); // use iced mpsc?

        let update_rate = 500; // min is 250 i think

        let btm_config = bottom::app::AppConfigFields {
            update_rate,
            temperature_type: TemperatureType::Celsius,
            show_average_cpu: true,
            use_dot: true,
            cpu_left_legend: true,
            use_current_cpu_total: true,
            unnormalized_cpu: false,
            use_basic_mode: false,
            default_time_value: 30 * 1000, // 30s
            time_interval: 1000,
            hide_time: true,
            autohide_time: false,
            use_old_network_legend: true,
            table_gap: 5,
            disable_click: true,
            enable_gpu: false,
            enable_cache_memory: false, // fixme: not sure what's this
            show_table_scroll_position: false,
            is_advanced_kill: false,
            memory_legend_position: None,
            network_legend_position: None,
            network_scale_type: bottom::app::AxisScaling::Linear,
            network_unit_type: bottom::utils::data_units::DataUnit::Bit,
            network_use_binary_prefix: false,
            retention_ms: 600000,
            dedicated_average_row: false,
        };

        // Set up the event loop thread.
        // Set it up early to speed up first access to data.
        let _collection_thread = create_collection_thread(
            tx.clone(),
            mpsc::channel().1, // ignore msg channel for now
            cancellation_token.clone(),
            &btm_config,
            DataFilters {
                disk_filter: None,
                mount_filter: None,
                temp_filter: None,
                net_filter: None,
            },
            bottom::app::layout_manager::UsedWidgets {
                use_cpu: true,
                use_mem: true,
                use_cache: true,
                use_gpu: true,
                use_net: true,
                use_proc: true,
                use_disk: true,
                use_temp: true,
                use_battery: true,
            },
        );

        Self {
            collector: Collector::new(rx),
            columns: vec![
                Column::new(ColumnKind::Name),
                Column::new(ColumnKind::Memory),
                Column::new(ColumnKind::Cpu),
                Column::new(ColumnKind::Pid),
                Column::new(ColumnKind::Command),
                Column::new(ColumnKind::Started),
                //
                // Column::new(ColumnKind::Index),
                // Column::new(ColumnKind::Category),
                // Column::new(ColumnKind::Enabled),
                // Column::new(ColumnKind::Notes),
                // Column::new(ColumnKind::Delete),
            ],
            rows: (0..300).map(Row::generate).collect(),
            header: scrollable::Id::unique(),
            body: scrollable::Id::unique(),
            resize_columns_enabled: true,
            min_width_enabled: true,
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

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::MinWidthEnabled(enabled) => self.min_width_enabled = enabled,
            Message::DarkThemeEnabled(enabled) => {
                if enabled {
                    self.theme = Theme::Dark;
                } else {
                    self.theme = Theme::Light;
                }
            }
            Message::CategoryChange(index, category) => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.category = category;
                }
            }
            Message::EnabledRow(index, is_enabled) => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.is_enabled = is_enabled;
                }
            }
            Message::EditNotes(index, notes) => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.notes = notes;
                }
            }
            Message::DeleteRow(index) => {
                self.rows.remove(index);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
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
            if self.min_width_enabled {
                table = table.min_width(size.width);
            }

            table.into()
        });

        let content = column![
            checkbox("Resize Columns", self.resize_columns_enabled,)
                .on_toggle(Message::ResizeColumnsEnabled),
            checkbox("Min Width", self.min_width_enabled,).on_toggle(Message::MinWidthEnabled),
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

            ColumnKind::Index => 10.0,    // 60.0,
            ColumnKind::Category => 10.0, // 100.0,
            ColumnKind::Enabled => 10.0,  // 155.0,
            ColumnKind::Notes => 10.0,    // 400.0,
            ColumnKind::Delete => 10.0,   // 100.0,
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
    Category,
    Enabled,
    Notes,
    Delete,
}

/// Actual storage for data row.
struct Row {
    program_name: String,
    pid: usize,
    command: String,

    notes: String,
    category: Category,
    is_enabled: bool,
}

impl Row {
    fn generate(index: usize) -> Self {
        let category = match index % 5 {
            0 => Category::A,
            1 => Category::B,
            2 => Category::C,
            3 => Category::D,
            4 => Category::E,
            _ => unreachable!(),
        };
        let is_enabled = index % 5 < 4;

        Self {
            program_name: "killa".to_string(),
            pid: 42 + index,
            command: "killa --version".to_string(),

            notes: String::new(),
            category,
            is_enabled,
        }
    }
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
            ColumnKind::Category => "Category",
            ColumnKind::Enabled => "Enabled",
            ColumnKind::Notes => "Notes",
            ColumnKind::Delete => "",
        };

        container(text(content)).center_y(24).into()
    }

    fn cell(&'a self, _col_index: usize, row_index: usize, row: &'a Row) -> Element<'a, Message> {
        let content: Element<_> = match self.kind {
            ColumnKind::Name => text!("{}", row.program_name).into(),
            ColumnKind::Memory => text!("{} Mb", 100).into(),
            ColumnKind::Cpu => text!("{} %", 10.0).into(),
            ColumnKind::Pid => text!("{}", row.pid).into(),
            ColumnKind::Command => text!("{}", row.command).into(),
            ColumnKind::Started => text!("{:?}", Instant::now()).into(),

            ColumnKind::Index => text(row_index).into(),
            ColumnKind::Category => pick_list(Category::ALL, Some(row.category), move |category| {
                Message::CategoryChange(row_index, category)
            })
            .into(),
            ColumnKind::Enabled => checkbox("", row.is_enabled)
                .on_toggle(move |enabled| Message::EnabledRow(row_index, enabled))
                .into(),
            ColumnKind::Notes => text_input("", &row.notes)
                .on_input(move |notes| Message::EditNotes(row_index, notes))
                .width(Length::Fill)
                .into(),
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
