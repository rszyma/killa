use iced::widget::{button, center, column, text};
use iced::{system, widget, Element, Task, Theme};

use crate::Message;
use bytesize::ByteSize;

pub(crate) fn system_info_element(information: &system::Information) -> Element<Message> {
    let element = {
        let system_name = text!(
            "System name: {}",
            information
                .system_name
                .as_ref()
                .unwrap_or(&"unknown".to_string())
        );

        let system_kernel = text!(
            "System kernel: {}",
            information
                .system_kernel
                .as_ref()
                .unwrap_or(&"unknown".to_string())
        );

        let system_version = text!(
            "System version: {}",
            information
                .system_version
                .as_ref()
                .unwrap_or(&"unknown".to_string())
        );

        let system_short_version = text!(
            "System short version: {}",
            information
                .system_short_version
                .as_ref()
                .unwrap_or(&"unknown".to_string())
        );

        let cpu_brand = text!("Processor brand: {}", information.cpu_brand);

        let cpu_cores = text!(
            "Processor cores: {}",
            information
                .cpu_cores
                .map_or("unknown".to_string(), |cores| cores.to_string())
        );

        let memory_readable = ByteSize::b(information.memory_total).to_string();

        let memory_total = text!(
            "Memory (total): {} bytes ({memory_readable})",
            information.memory_total,
        );

        let memory_text = if let Some(memory_used) = information.memory_used {
            let memory_readable = ByteSize::b(memory_used).to_string();

            format!("{memory_used} bytes ({memory_readable})")
        } else {
            String::from("None")
        };

        let memory_used = text!("Memory (used): {memory_text}");

        let graphics_adapter = text!("Graphics adapter: {}", information.graphics_adapter);

        let graphics_backend = text!("Graphics backend: {}", information.graphics_backend);

        const FONTSIZE: u16 = 13;

        column![
            system_name.size(FONTSIZE),
            system_kernel.size(FONTSIZE),
            system_version.size(FONTSIZE),
            system_short_version.size(FONTSIZE),
            cpu_brand.size(FONTSIZE),
            cpu_cores.size(FONTSIZE),
            memory_total.size(FONTSIZE),
            memory_used.size(FONTSIZE),
            graphics_adapter.size(FONTSIZE),
            graphics_backend.size(FONTSIZE),
            button("Refresh").on_press(Message::Refresh)
        ]
        .spacing(3)
        .into()
    };
    element
}
