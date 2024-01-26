mod callback;
mod message;

use atri_core::Game;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::{constants::PER_PAGE, state::Search};
pub use callback::callback_handler;
pub use message::message_handler;

fn make_keyboard(search: &Search, games: Vec<Game>) -> InlineKeyboardMarkup {
  let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::with_capacity(PER_PAGE + 1);

  let mut opt_row: Vec<InlineKeyboardButton> = Vec::with_capacity(3);
  if search.offset != 0 {
    opt_row.push(InlineKeyboardButton::callback(
      "上一页",
      format!("p {}", search.id),
    ));
  }
  opt_row.push(InlineKeyboardButton::callback(
    "取消",
    format!("c {}", search.id),
  ));
  if games.len() == PER_PAGE + 1 {
    opt_row.push(InlineKeyboardButton::callback(
      "下一页",
      format!("s {}", search.id),
    ));
  }

  games.into_iter().take(PER_PAGE).for_each(|game| {
    keyboard.push(vec![InlineKeyboardButton::callback(
      game.name,
      format!("g {} {}", game.id, search.id),
    )]);
  });

  keyboard.push(opt_row);

  InlineKeyboardMarkup::new(keyboard)
}
