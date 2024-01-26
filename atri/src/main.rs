use anyhow::Result;
use teloxide::{
  adaptors::{throttle::Limits, CacheMe, Throttle},
  prelude::*,
  types::{InlineKeyboardButton, InlineKeyboardMarkup, Me, ParseMode},
  utils::{command::BotCommands, html::bold},
};
use url::Url;

use atri_core::{search, setup_tracing, tracing, Game, MeiliClient};

type AtriBot = CacheMe<Throttle<Bot>>;

#[tokio::main]
async fn main() {
  setup_tracing();

  let client = MeiliClient::new("http://localhost:7700", None::<&str>);

  let bot: AtriBot = Bot::from_env().throttle(Limits::default()).cache_me();

  let handler = dptree::entry()
    .branch(Update::filter_message().endpoint(message_handler))
    .branch(Update::filter_callback_query().endpoint(callback_handler));

  tracing::info!("Listening...");

  Dispatcher::builder(bot, handler)
    .dependencies(dptree::deps![client])
    .build()
    .dispatch()
    .await;
}

const PER_PAGE: usize = 10;

fn make_keyboard(
  games: Vec<Game>,
  keyword: impl AsRef<str>,
  offset: usize,
) -> InlineKeyboardMarkup {
  let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::with_capacity(PER_PAGE + 1);

  games.iter().take(PER_PAGE).for_each(|game| {
    keyboard.push(vec![InlineKeyboardButton::callback(
      &game.name,
      format!("g {}", game.id),
    )]);
  });

  let keyword = keyword.as_ref();

  let mut opt_row: Vec<InlineKeyboardButton> = Vec::with_capacity(3);
  if offset != 0 {
    opt_row.push(InlineKeyboardButton::callback(
      "上一页",
      format!("s {} {keyword}", offset - PER_PAGE),
    ));
  }
  opt_row.push(InlineKeyboardButton::callback("取消", "c"));
  if games.len() == PER_PAGE + 1 {
    opt_row.push(InlineKeyboardButton::callback(
      "下一页",
      format!("s {} {keyword}", offset + PER_PAGE),
    ));
  }

  keyboard.push(opt_row);

  InlineKeyboardMarkup::new(keyboard)
}

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

async fn message_handler(bot: AtriBot, client: MeiliClient, msg: Message, me: Me) -> Result<()> {
  let Some(text) = msg.text() else {
    return Ok(());
  };

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

      let Ok(res) = search(&client, &keyword, PER_PAGE + 1, 0).await else {
        bot
          .send_message(msg.chat.id, "搜索失败了！")
          .reply_to_message_id(msg.id)
          .await?;
        return Ok(());
      };

      tracing::info!(estimated_num = res.estimated_total_hits, "Search finished");

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
          res.hits.into_iter().map(|g| g.result).collect(),
          &keyword,
          0,
        ))
        .await?;
    }

    Err(_) => {
      // ignore
    }
  }

  Ok(())
}

async fn callback_handler(bot: AtriBot, client: MeiliClient, q: CallbackQuery) -> Result<()> {
  let Some(data) = q.data else {
    tracing::error!("No data callback");
    return Ok(());
  };

  let Some(msg) = q.message else {
    tracing::error!("No message callback");
    return Ok(());
  };

  let data: Vec<_> = data.split(' ').collect();

  match data[0] {
    "s" => {
      bot.answer_callback_query(q.id).await?;
    }
    "c" => {
      bot.answer_callback_query(q.id).await?;
      bot
        .edit_message_text(msg.chat.id, msg.id, "已取消。")
        .await?;
    }
    "g" => {
      bot.answer_callback_query(q.id).await?;

      let Ok(res) = search(&client, data[1], 1, 0).await else {
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
        .reply_markup(InlineKeyboardMarkup::new([[InlineKeyboardButton::url(
          "点击下载",
          download_link,
        )]]))
        .await?;
    }
    _ => {
      tracing::error!("Unknown data callback");
    }
  }

  Ok(())
}
