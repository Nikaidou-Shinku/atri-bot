use std::sync::Arc;

use anyhow::Result;
use atri_core::{search, tracing};
use teloxide::{
  prelude::*,
  types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
  utils::html::bold,
};
use url::Url;

use super::make_keyboard;
use crate::{
  state::{AtriState, SearchMode},
  AtriBot,
};

pub async fn callback_handler(bot: AtriBot, state: Arc<AtriState>, q: CallbackQuery) -> Result<()> {
  let Some(data) = q.data else {
    tracing::error!("No data callback");
    return Ok(());
  };

  let Some(msg) = q.message else {
    tracing::error!("No message callback");
    return Ok(());
  };

  let data: Vec<_> = data.split(' ').collect();

  if data.len() < 2 {
    tracing::error!(?data, "Unknown callback data");
    return Ok(());
  }

  match data[0] {
    "c" => {
      bot.answer_callback_query(q.id).await?;
      let search_id: usize = data[1].parse()?;
      state.drop_search(search_id);
      bot
        .edit_message_text(msg.chat.id, msg.id, "已取消。")
        .await?;
    }
    "g" => {
      if data.len() != 3 {
        tracing::error!(?data, "Unknown callback data");
        return Ok(());
      }

      bot.answer_callback_query(q.id).await?;

      let Ok(res) = search(&state.meili_client, data[1], 1, 0).await else {
        tracing::error!(id = data[1], "Find game error");
        bot
          .edit_message_text(msg.chat.id, msg.id, "检索游戏时出错！")
          .await?;
        return Ok(());
      };

      if res.hits.is_empty() {
        tracing::error!(id = data[1], "Can not find game");
        bot
          .edit_message_text(msg.chat.id, msg.id, "检索游戏时出错！")
          .await?;
        return Ok(());
      }

      let game = &res.hits[0].result;

      let Ok(download_link) = Url::parse(&format!(
        "https://www.shinnku.com/api/download/{}",
        game.paths.join("/")
      )) else {
        tracing::error!(id = data[1], "Can not parse download link");
        bot
          .edit_message_text(msg.chat.id, msg.id, "生成下载链接时出错！")
          .await?;
        return Ok(());
      };

      bot
        .edit_message_text(
          msg.chat.id,
          msg.id,
          format!("{}\n文件大小：{}", bold(&game.name), game.size),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new([
          [InlineKeyboardButton::url("点击下载", download_link)],
          [InlineKeyboardButton::callback(
            "返回",
            format!("r {}", data[2]),
          )],
        ]))
        .await?;
    }
    "p" | "r" | "s" => {
      bot.answer_callback_query(q.id).await?;
      let search_id: usize = data[1].parse()?;
      let search_mode = match data[0] {
        "p" => SearchMode::PrevPage,
        "r" => SearchMode::Direct,
        "s" => SearchMode::NextPage,
        _ => unreachable!(),
      };

      tracing::info!(id = search_id, mode = ?search_mode, "Start re-searching...");

      let Ok((search, res)) = state.continue_search(search_id, search_mode).await else {
        bot
          .edit_message_text(msg.chat.id, msg.id, "搜索失败了！")
          .await?;
        return Ok(());
      };

      tracing::info!(
        id = search_id,
        estimated_num = res.estimated_total_hits,
        "Re-search finished"
      );

      if res.hits.is_empty() {
        bot
          .edit_message_text(msg.chat.id, msg.id, "什么都没有找到！")
          .await?;
        return Ok(());
      }

      bot
        .edit_message_text(
          msg.chat.id,
          msg.id,
          if let Some(num) = res.estimated_total_hits {
            format!("搜索完成！找到约 {} 个结果！", num)
          } else {
            "搜索完成！".to_owned()
          },
        )
        .reply_markup(make_keyboard(
          &search,
          res.hits.into_iter().map(|g| g.result).collect(),
        ))
        .await?;
    }
    _ => {
      tracing::error!(?data, "Unknown callback data");
      return Ok(());
    }
  }

  Ok(())
}
