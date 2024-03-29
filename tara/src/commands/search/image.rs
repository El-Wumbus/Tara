use std::{collections::HashMap, sync::Arc};

use component_macro::component;
use once_cell::sync::Lazy;
use serenity::{
    all::{ChannelId, CommandInteraction, ComponentInteraction, MessageId, ReactionType, UserId},
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, EditInteractionResponse,
    },
    client::Cache,
    http::Http,
};
use tokio::sync::Mutex;

use crate::componet::Component;

pub(super) type Umid = (ChannelId, MessageId);

#[allow(clippy::type_complexity)]
pub(super) static IMAGE_RESULTS: Lazy<
    Arc<Mutex<HashMap<Umid, (Vec<unsplash::UnsplashImage>, usize, Arc<CommandInteraction>)>>>,
> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(super) static USERS: Lazy<Arc<Mutex<HashMap<UserId, Umid>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

use crate::commands::{common::unsplash, CommandArguments};

#[component(buttons_cleanup_handler)]
pub(super) async fn forward_button_handler(
    interaction: ComponentInteraction,
    args: CommandArguments,
) -> anyhow::Result<()> {
    button_handler(interaction, args, |x| x + 1).await
}

#[component(buttons_cleanup_handler)]
pub(super) async fn backward_button_handler(
    interaction: ComponentInteraction,
    args: CommandArguments,
) -> anyhow::Result<()> {
    button_handler(interaction, args, |x| x - 1).await
}

async fn button_handler(
    component: ComponentInteraction,
    args: CommandArguments,
    f: fn(isize) -> isize,
) -> anyhow::Result<()> {
    let Some((channel_id, message_id)) = USERS.lock().await.get(&component.user.id).copied() else {
        return Ok(());
    };
    if channel_id != component.channel_id || message_id != component.message.id {
        // Do nothing because this user didn't start this interaction and shouldn't be able to
        // interfere with the others.
        return Ok(());
    }


    let mut lock = IMAGE_RESULTS.lock().await;
    let (imgs, mut i, _) = lock.get(&(component.channel_id, component.message.id)).unwrap();
    let mut x = f(i as isize);

    if x >= imgs.len() as isize {
        x = 0;
    } else if x < 0 {
        x = imgs.len() as isize - 1;
    }
    i = x as usize;

    let id = format!("{}-{}", component.channel_id, component.message.id);
    let components = button_components(&id, i, imgs.len(), false);

    let image = imgs.get(i).unwrap();
    let embed: CreateEmbed = image.into();
    component
        .create_response(
            &args.context.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(components),
            ),
        )
        .await?;

    let (_, ref mut n, _) = lock
        .get_mut(&(component.channel_id, component.message.id))
        .unwrap();
    *n = i;

    Ok(())
}

pub(super) async fn buttons_cleanup_handler(
    id: String,
    http: Arc<Http>,
    _cache: Arc<Cache>,
) -> anyhow::Result<()> {
    let (channel_id, message_id, _) = sscanf::sscanf!(id, "{u64}-{u64}-{str}").unwrap();
    let (channel_id, message_id) = (ChannelId::new(channel_id), MessageId::new(message_id));

    if let Some((imgs, i, command)) = IMAGE_RESULTS.lock().await.remove(&(channel_id, message_id)) {
        let message = command.get_response(&http).await?;
        let id = format!("{}-{}", command.channel_id, message.id);
        let components = button_components(&id, i, imgs.len(), true);

        command
            .edit_response(
                &http,
                EditInteractionResponse::new()
                    .components(components)
                    .content("Disabled"),
            )
            .await?;
    }
    Ok(())
}

pub(super) fn button_components(
    id: &str,
    current_item: usize,
    imgs_len: usize,
    disabled: bool,
) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{id}-prev"))
            .emoji(ReactionType::Unicode(String::from("⬅️")))
            .disabled(disabled),
        CreateButton::new(format!("{id}-next"))
            .emoji(ReactionType::Unicode(String::from("➡️")))
            .disabled(disabled)
            .label(format!("Next ({}/{imgs_len})", current_item + 1)),
    ])]
}
