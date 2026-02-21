use crate::ui::ColumnKind;

/// Messages that update UI.
#[derive(Debug, Clone)]
pub enum Message {
    CollectedData(Box<bottom::data_collection::Data>),
    // SyncHeader(scrollable::AbsoluteOffset),
    // Resizing(usize, f32),
    // Resized,
    ToggleFreeze,
    Freeze(bool),
    ToggleWireframe(bool),
    SetSortField(ColumnKind),
    Search(TextInputAction),
    StageSignalAllFiltered(rustix::process::Signal),
    /// Escape key or back button pressed.
    Back,
    /// Enter pressed.
    Enter,
    SystemThemeChanged(iced::theme::Mode),
}

#[derive(Debug, Clone)]
pub enum TextInputAction {
    Append(String),
    PopLastChar,
    Replace(String),
    Toggle,
    Hide,
}
