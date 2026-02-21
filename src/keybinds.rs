use crate::ui::{ColumnKind, Message, TextInputAction};
use iced::{Event, Subscription, event, keyboard};

pub fn handle_keybinds() -> Subscription<Message> {
    event::listen_with(|event, status, _window| -> Option<Message> {
        use iced::keyboard::Modifiers as M;
        use keyboard::Key as T;
        use keyboard::key::Named as K;

        // Right now, there's only one widget that can capture input - seach box.
        let is_search_box_active = matches!(status, event::Status::Captured);

        let (key, modifiers) =
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                (key, modifiers)
            } else {
                return None;
            };

        const NO_MODS: iced::keyboard::Modifiers = M::empty();
        const CTRL_SHIFT: iced::keyboard::Modifiers = M::CTRL.union(M::SHIFT);

        // keybinds
        let res: Option<_> = match (modifiers, key.as_ref(), is_search_box_active) {
            (_, T::Named(K::Escape), _) => Some(Message::Back),
            (_, T::Named(K::Enter), _) => Some(Message::Enter),

            ////////////////////////
            // search
            (NO_MODS, T::Character(c), false) => {
                Some(Message::Search(TextInputAction::Append(c.to_string())))
            }
            // handle only if not active, otherwise we get double backspace.
            (NO_MODS, T::Named(K::Backspace), false) => {
                Some(Message::Search(TextInputAction::PopLastChar))
            }

            (M::CTRL, T::Character("f"), true) => Some(Message::Search(TextInputAction::Hide)),
            (M::CTRL, T::Character("f"), false) => Some(Message::Search(TextInputAction::Toggle)),

            (M::CTRL, T::Character("j"), _) => Some(Message::Freeze(true)),
            (CTRL_SHIFT, T::Character("j"), _) => Some(Message::Freeze(false)),

            ////////////////////////
            // select sort field
            (M::CTRL, T::Character("1"), _) => Some(Message::SetSortField(ColumnKind::Cpu)),
            (M::CTRL, T::Character("2"), _) => Some(Message::SetSortField(ColumnKind::Memory)),
            (M::CTRL, T::Character("3"), _) => Some(Message::SetSortField(ColumnKind::Pid)),

            // other
            (M::CTRL, T::Character("k"), _) => Some(Message::StageSignalAllFiltered(
                rustix::process::Signal::Term,
            )),
            (CTRL_SHIFT, T::Character("k"), _) => Some(Message::StageSignalAllFiltered(
                rustix::process::Signal::Kill,
            )),

            _ => None,
        };

        res
    })
}
