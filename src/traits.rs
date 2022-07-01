use crate::App;
use std::any::Any;

pub trait EditingPopUp {
    fn set_editing(&mut self, state: bool);
    fn is_editing(&self) -> bool;
    fn select_next(&mut self);
    fn select_prev(&mut self);
    fn selected_buf(&mut self) -> &mut String;
    fn submit(&self, app: &mut App) -> Result<(), &'static str>;
    fn render(&self) -> Vec<tui::widgets::Paragraph<'_>>;
    fn popup_type(&self) -> crate::app::PopUpType;
    fn as_any(&self) -> &dyn Any;
}
