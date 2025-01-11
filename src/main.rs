use std::fmt::Display;

use iced::widget::{button, center, column, text};
use iced::{system, widget, Alignment, Element, Task, Theme};

mod sysinfo;

pub fn main() -> iced::Result {
    iced::application("fsm / fastsm", App::update, App::view)
        .theme(|_| Theme::Dark)
        .run_with(App::new)
}

#[derive(Default)]
#[allow(clippy::large_enum_variant)]
enum App {
    #[default]
    Loading,
    Loaded {
        sys_info: system::Information,
        // table: iced_table::Table<'a, _>,
    },
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
enum Message {
    InformationReceived(system::Information),
    Refresh,
    Table(TableMessage),
}

#[derive(Debug, Clone)]
enum TableMessage {
    ProduceResults,
    /// refresh table with new data
    RefreshResults(prettytable::Table),
    // SyncHeader(scrollable::AbsoluteOffset),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self::Loading,
            system::fetch_information().map(Message::InformationReceived),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => {
                let (state, refresh) = Self::new();
                *self = state;
                refresh
            }
            Message::InformationReceived(sys_info) => {
                *self = Self::Loaded { sys_info };
                Task::none()
            }
            Message::Table(t) => match t {
                TableMessage::ProduceResults => todo!(),
                TableMessage::RefreshResults(_) => todo!(),
                TableMessage::SyncHeader(_) => todo!(),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        // let content: Element<_> = match self {
        //     App::Loading => text("Loading...").size(25).into(),
        //     App::Loaded {
        //         sys_info: information,
        //     } => sysinfo::system_info_element(information),
        // };
        // center(content).into()

        widget::container(widget::scrollable(|_| {
            let results_table: iced_table::Table<
                '_,
                TableColumn,
                TableRow<String>,
                TableMessage,
                Theme,
            > = iced_table::table(
                self.results_header.clone(),
                self.results_body.clone(),
                &self.results_titles,
                &self.results_rows,
                TableMessage::SyncHeader,
            );
            results_table.into()
        }))
        .into()
    }

    // fn view(&self) -> Element<'_, Self::Message, Self::Theme, iced::Renderer> {
    //     // let content =
    //     //     column![results_table].spacing(20);
    // }
}

// TODO: Create wrappers around prettytable::Row
//       to correspond to Column and Row required by iced_table
#[derive(Clone)]
struct TableColumn {
    kind: TableColumnKind,
    align: Alignment,
    width: f32,
    resize_offset: Option<f32>,
} // Hiding columns requires a wrapper around `type Row = Vec<GUICell<String>>`

struct TableRow<T: Display> {
    data: Vec<TableCell<T>>,
}
impl<S: Into<String>, I: IntoIterator<Item = S>> From<I> for TableRow<String> {
    fn from(value: I) -> Self {
        Self {
            data: value
                .into_iter()
                .map(Into::into)
                .map(TableCell::from)
                .collect_vec(),
        }
    }
}

// impl From<prettytable::Row> for GUIRow<String> {
//     fn from(value: prettytable::Row) -> Self {
//         value.into_iter()
//     }
// }

#[derive(Clone)]
enum TableColumnKind {
    Index(String), // For displaying text
}

impl TableColumn {
    fn new(kind: TableColumnKind, alignment: Option<Alignment>) -> Self {
        let width = match kind {
            TableColumnKind::Index(_) => 60.0,
        };

        let align = if let Some(a) = alignment {
            a
        } else {
            Alignment::End
        };
        Self {
            kind,
            align,
            width,
            resize_offset: None,
        }
    }
}

impl<S: ToString> From<S> for TableColumn {
    fn from(value: S) -> Self {
        TableColumn::new(TableColumnKind::Index(value.to_string()), None)
    }
}

#[derive(Clone)]
struct TableCell<T: Display> {
    content: T,
    align: Alignment, // TODO: Find a way to retrieve alignment
}
