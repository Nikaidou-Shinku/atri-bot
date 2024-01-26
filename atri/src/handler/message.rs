use std::sync::Arc;

use anyhow::Result;
use atri_core::tracing;
use teloxide::{prelude::*, types::Me, utils::command::BotCommands};

use super::make_keyboard;
use crate::{AtriBot, AtriState};

pub async fn message_handler(
  bot: AtriBot,
  state: Arc<AtriState>,
  msg: Message,
  me: Me,
) -> Result<()> {
  let Some(text) = msg.text() else {
    if let Some(user) = msg.from() {
      tracing::info!(from = user.full_name(), "Non text message");
    }
    return Ok(());
  };

  #[derive(BotCommands)]
  #[command(
    rename_rule = "lowercase",
    description = "アトリは、次のような機能があります！"
  )]
  enum Command {
    #[command(description = "あいさつ")]
    Start,
    #[command(description = "このヘルプテキストを表示する")]
    Help,
    #[command(description = "www.shinnku.com を検索する")]
    Shinnku(String),
  }

  match BotCommands::parse(text, me.username()) {
    Ok(Command::Start) => {
      bot
        .send_message(msg.chat.id, "アトリは、高性能ですから！")
        .reply_to_message_id(msg.id)
        .await?;
    }

    Ok(Command::Help) => {
      bot
        .send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_to_message_id(msg.id)
        .await?;
    }

    Ok(Command::Shinnku(keyword)) => {
      if keyword.is_empty() {
        bot
          .send_message(msg.chat.id, "请在指令后附带要搜索的关键字！")
          .reply_to_message_id(msg.id)
          .await?;
        return Ok(());
      }

      tracing::info!(keyword, "Start searching...");

      let Ok((search, res)) = state.new_search(&keyword).await else {
        bot
          .send_message(msg.chat.id, "搜索失败了！")
          .reply_to_message_id(msg.id)
          .await?;
        return Ok(());
      };

      tracing::info!(
        keyword,
        estimated_num = res.estimated_total_hits,
        "Search finished"
      );

      if res.hits.is_empty() {
        bot
          .send_message(msg.chat.id, "什么都没有找到！")
          .reply_to_message_id(msg.id)
          .await?;
        return Ok(());
      }

      bot
        .send_message(
          msg.chat.id,
          if let Some(num) = res.estimated_total_hits {
            format!("搜索完成！找到约 {} 个结果！", num)
          } else {
            "搜索完成！".to_owned()
          },
        )
        .reply_to_message_id(msg.id)
        .reply_markup(make_keyboard(
          &search,
          res.hits.into_iter().map(|g| g.result).collect(),
        ))
        .await?;
    }

    Err(error) => {
      if let Some(user) = msg.from() {
        tracing::info!(%error, from = user.full_name(), text, "Non command message");
      }
      return Ok(());
    }
  }

  Ok(())
}
