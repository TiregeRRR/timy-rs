use chrono::{DateTime, Duration, TimeDelta, Utc};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    types::{KeyboardButton, KeyboardMarkup},
    utils::command::BotCommands,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveTargetHours,
    Rest {
        target_hours: Duration,
        cur_hours: Duration,
    },
    Work {
        target_hours: Duration,
        cur_hours: Duration,
        work_start: DateTime<Utc>,
    },
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start tracking work.")]
    Work,
    #[command(description = "stop tracking work.")]
    Rest,
    #[command(description = "show current status.")]
    Status,
    #[command(description = "reset working time.")]
    Reset,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting purchase bot...");

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Start].branch(case![Command::Help].endpoint(help)))
        .branch(
            case![State::Rest {
                target_hours,
                cur_hours
            }]
            .branch(case![Command::Work].endpoint(work)),
        )
        .branch(
            case![State::Rest {
                target_hours,
                cur_hours
            }]
            .branch(case![Command::Status].endpoint(rest_status)),
        )
        .branch(
            case![State::Work {
                target_hours,
                cur_hours,
                work_start,
            }]
            .branch(case![Command::Rest].endpoint(rest)),
        )
        .branch(
            case![State::Work {
                target_hours,
                cur_hours,
                work_start,
            }]
            .branch(case![Command::Status].endpoint(work_status)),
        );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveTargetHours].endpoint(receive_target_hours))
        .branch(dptree::endpoint(invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler)
}

async fn help(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;

    dialogue.update(State::ReceiveTargetHours).await?;

    Ok(())
}

async fn work(
    bot: Bot,
    dialogue: MyDialogue,
    target_and_cur_hours: (Duration, Duration),
    msg: Message,
) -> HandlerResult {
    let work_start = chrono::offset::Utc::now();

    let actions = ["/rest", "/status"].map(|action| KeyboardButton::new(action));

    bot.send_message(msg.chat.id, format!("Started work at {work_start}"))
        .reply_markup(KeyboardMarkup::new([actions]))
        .await?;

    dialogue
        .update(State::Work {
            target_hours: target_and_cur_hours.0,
            cur_hours: target_and_cur_hours.1,
            work_start: work_start,
        })
        .await?;

    Ok(())
}

async fn rest(
    bot: Bot,
    dialogue: MyDialogue,
    target_cur_work_hours: (Duration, Duration, DateTime<Utc>),
    msg: Message,
) -> HandlerResult {
    let cur_time = chrono::offset::Utc::now();
    let time_delta = cur_time.signed_duration_since(target_cur_work_hours.2);
    let new_cur_hours = target_cur_work_hours.1 + time_delta;

    let actions = ["/work", "/status"].map(|action| KeyboardButton::new(action));

    let seconds = new_cur_hours.num_seconds() % 60;
    let minutes = (new_cur_hours.num_seconds() / 60) % 60;
    let hours = (new_cur_hours.num_seconds() / 60) / 60;

    let target_seconds = target_cur_work_hours.0.num_seconds() % 60;
    let target_minutes = (target_cur_work_hours.0.num_seconds() / 60) % 60;
    let target_hours = (target_cur_work_hours.0.num_seconds() / 60) / 60;

    bot.send_message(
        msg.chat.id,
        format!(
            "Work done. Done {:0>2}:{:0>2}:{:0>2} of {:0>2}:{:0>2}:{:0>2}",
            hours, minutes, seconds, target_hours, target_minutes, target_seconds,
        ),
    )
    .reply_markup(KeyboardMarkup::new([actions]))
    .await?;

    dialogue
        .update(State::Rest {
            target_hours: target_cur_work_hours.0,
            cur_hours: new_cur_hours,
        })
        .await?;

    Ok(())
}

async fn work_status(
    bot: Bot,
    target_cur_work_hours: (Duration, Duration, DateTime<Utc>),
    msg: Message,
) -> HandlerResult {
    let cur_time = chrono::offset::Utc::now();
    let time_delta = cur_time.signed_duration_since(target_cur_work_hours.2);
    let new_cur_hours = target_cur_work_hours.1 + time_delta;

    let seconds = new_cur_hours.num_seconds() % 60;
    let minutes = (new_cur_hours.num_seconds() / 60) % 60;
    let hours = (new_cur_hours.num_seconds() / 60) / 60;

    let target_seconds = target_cur_work_hours.0.num_seconds() % 60;
    let target_minutes = (target_cur_work_hours.0.num_seconds() / 60) % 60;
    let target_hours = (target_cur_work_hours.0.num_seconds() / 60) / 60;

    bot.send_message(
        msg.chat.id,
        format!(
            "Done {:0>2}:{:0>2}:{:0>2} of {:0>2}:{:0>2}:{:0>2}",
            hours, minutes, seconds, target_hours, target_minutes, target_seconds,
        ),
    )
    .await?;

    Ok(())
}

async fn rest_status(
    bot: Bot,
    target_cur_hours: (Duration, Duration),
    msg: Message,
) -> HandlerResult {
    let seconds = target_cur_hours.1.num_seconds() % 60;
    let minutes = (target_cur_hours.1.num_seconds() / 60) % 60;
    let hours = (target_cur_hours.1.num_seconds() / 60) / 60;

    let target_seconds = target_cur_hours.0.num_seconds() % 60;
    let target_minutes = (target_cur_hours.0.num_seconds() / 60) % 60;
    let target_hours = (target_cur_hours.0.num_seconds() / 60) / 60;

    bot.send_message(
        msg.chat.id,
        format!(
            "Done {:0>2}:{:0>2}:{:0>2} of {:0>2}:{:0>2}:{:0>2}",
            hours, minutes, seconds, target_hours, target_minutes, target_seconds,
        ),
    )
    .await?;

    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}

async fn receive_target_hours(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(target_hours_str) => {
            let Ok(target_hours) = target_hours_str.to_string().parse::<i64>() else {
                bot.send_message(msg.chat.id, "Send correct hours count.")
                    .await?;

                return Ok(());
            };

            let actions = ["/work", "/status"].map(|action| KeyboardButton::new(action));

            bot.send_message(msg.chat.id, "Setup done.")
                .reply_markup(KeyboardMarkup::new([actions]))
                .await?;

            dialogue
                .update(State::Rest {
                    target_hours: TimeDelta::hours(target_hours),
                    cur_hours: TimeDelta::hours(0),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send correct hours count.")
                .await?;
        }
    }

    Ok(())
}
