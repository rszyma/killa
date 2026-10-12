use crate::Message;
use iced::widget::tooltip::Position;
use iced::widget::{container, text, tooltip};
use iced::{Element, Length, Pixels, Renderer, Theme};
use std::fmt::Display;
use std::time::Duration;

/// Actual storage for data row.
#[derive(Clone, Debug)]
pub struct Row {
    //  todo: get this out?
    pub row_index: usize, // todo: get this out?
    pub program_name: String,
    pub program_name_lowercase: String, // index for search
    pub mem: u64,
    pub cpu_perc: f32,
    pub pid: i32,
    pub command: String,
    pub command_lowercase: String, // index for search
    pub cpu_time: Duration,
    pub uptime: Duration,
    // start_time: Option<chrono::DateTime<chrono::Local>>,
}

impl Row {
    pub fn cell(self, for_column: &ColumnKind) -> Element<'_, Message, Theme, Renderer> {
        let font_size = Pixels::from(13.0);
        let tooltip_font_size = Pixels::from(11.0);
        let content: Element<_> = match for_column {
            ColumnKind::Name => text!("{}", self.program_name).size(font_size).into(),
            ColumnKind::Memory => text!("{} MB", self.mem).size(font_size).into(),
            ColumnKind::Cpu => if self.cpu_perc != 0.0 {
                text!("{:.1} %", self.cpu_perc)
            } else {
                text!("—")
            }
            .size(font_size)
            .into(),
            ColumnKind::Pid => text!("{}", self.pid).size(font_size).into(),
            ColumnKind::Command => tooltip(
                text!("{}", self.command).size(font_size),
                container(text!("{}", self.command).size(tooltip_font_size))
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
            ColumnKind::CpuTime => if self.cpu_time.is_zero() {
                text!("—")
            } else {
                text!("{}", humantime::format_duration(self.cpu_time))
            }
            .size(font_size)
            .into(),
            ColumnKind::Uptime => if self.uptime.is_zero() {
                text!("—")
            } else {
                text!("{}", humantime::format_duration(self.uptime))
            }
            .size(font_size)
            .into(),
        };

        container(content)
            .width(Length::Fill)
            .center_y(15)
            .clip(true)
            .into()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColumnKind {
    Name,
    Memory,
    Cpu,
    Pid,
    Command,
    Started,
    CpuTime,
    Uptime,
}

impl Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::Name => f.write_str("Name"),
            ColumnKind::Memory => f.write_str("Memory"),
            ColumnKind::Cpu => f.write_str("CPU"),
            ColumnKind::Pid => f.write_str("ID"),
            ColumnKind::CpuTime => f.write_str("Time"),
            ColumnKind::Uptime => f.write_str("Uptime"),
            ColumnKind::Started => f.write_str("Started"),
            ColumnKind::Command => f.write_str("Command"),
        }
    }
}

impl ColumnKind {
    pub fn width_ratio(&self) -> f32 {
        match self {
            ColumnKind::Name => 0.35,
            ColumnKind::Memory => 0.1,
            ColumnKind::Cpu => 0.06,
            ColumnKind::Pid => 0.08,
            ColumnKind::Command => 0.3,
            ColumnKind::CpuTime => 0.1,
            ColumnKind::Uptime => 0.1,
            ColumnKind::Started => 0.1,
        }
    }

    /// Canonical position, keeps column order stable when toggling visibility.
    pub fn rank(&self) -> u8 {
        match self {
            ColumnKind::Name => 0,
            ColumnKind::Memory => 1,
            ColumnKind::Cpu => 2,
            ColumnKind::Pid => 3,
            ColumnKind::CpuTime => 4,
            ColumnKind::Uptime => 5,
            ColumnKind::Started => 6,
            ColumnKind::Command => 7,
        }
    }
}
