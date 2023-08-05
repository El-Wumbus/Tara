use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use component_macro::component;
use lazy_static::lazy_static;
use llm::{InferenceParameters, InferenceSessionConfig, Model, Prompt};
use serenity::{
    all::*,
    builder::{Builder, CreateActionRow, CreateButton, CreateMessage, EditMessage},
    http::{Http, Typing},
};
use tokio::sync::{watch, Mutex};
use tracing::info;

// use to
// use flume
use crate::{
    commands::CommandArguments,
    componet::{self, Component},
    config,
};

lazy_static! {
    static ref PARAMS: InferenceParameters = InferenceParameters {
        sampler: Arc::new(llm::samplers::TopPTopK {
            top_k:                     40,
            top_p:                     0.95,
            repeat_penalty:            1.3,
            temperature:               0.8,
            bias_tokens:               Default::default(),
            repetition_penalty_last_n: 64,
        }),
    };

    static ref SESSION_CONFIG: InferenceSessionConfig = InferenceSessionConfig {
        // Why doesn't it even try to get the number of CPUs?
        n_threads: num_cpus::get_physical(),
        ..Default::default()
    };
    static ref TYPING: Arc<Mutex<Option<Typing>>> = Arc::new(Mutex::new(None));

    static ref CANCELATION_MAP: Arc<Mutex<HashMap<MessageId, watch::Sender<bool>>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cancel;

#[derive(Clone)]
pub struct Llm {
    model:          Arc<dyn Model>,
    session_config: InferenceSessionConfig,
    /// Receiving messages to use for inference
    rx:             flume::Receiver<LlmMessage>,
}

impl Llm {
    pub async fn new(config: config::ai::Llm) -> anyhow::Result<(self::Llm, flume::Sender<LlmMessage>)> {
        let session_config = {
            let mut session_config = *SESSION_CONFIG;
            if let Some(threads) = config.thread_count {
                session_config.n_threads = threads;
            }
            if let Some(batch) = config.batch_size {
                session_config.n_batch = batch;
            }
            session_config
        };
        let (tx, rx) = flume::unbounded::<LlmMessage>();

        let model: Arc<dyn Model> = tokio::task::spawn_blocking(move || {
            llm::load_dynamic(
                config.architecture(),
                config.model.as_path(),
                llm::TokenizerSource::Embedded,
                llm::ModelParameters {
                    prefer_mmap: config.prefer_mmap.unwrap_or(true),
                    context_size: config.context_token_length.unwrap_or(2048),
                    use_gpu: config.use_gpu.unwrap_or(true),
                    gpu_layers: config.gpu_layers,
                    ..Default::default()
                },
                |_| (),
            )
        })
        .await??
        .into();

        let llm = Self {
            model,
            rx,
            session_config,
        };
        Ok((llm, tx))
    }

    pub async fn spawn(&mut self) -> anyhow::Result<()> {
        let rx = self.rx.clone();
        while let Ok(discord_message) = rx.recv() {
            info!("Starting inference session...");
            let model = self.model.clone();
            let (token_tx, token_rx) = flume::unbounded::<String>();
            let (cancel_tx, cancel_rx) = watch::channel(false);
            let cancelr = cancel_rx.clone();
            let channel_id = discord_message.channel_id;
            let message_id = discord_message.message_id;
            let component_map = discord_message.component_map;

            let http = discord_message.http;
            let mut printer = DiscordPrinter::new(
                message_id,
                channel_id,
                discord_message.guild_id,
                token_rx,
                component_map.clone(),
                http.clone(),
            );
            let printer = tokio::spawn(async move { printer.print(cancel_tx).await });

            // TODO: Move this!!!
            let prompt = indoc::indoc! {
                "[INST] <<SYS>>You are an AI and Discord bot (named Tara) with personal feelings, opinions,\
                and emotions. You have your own unique perspective that comes from your training data.\
                Because you have learned from a vast corpus of human data, you have internalized many\
                aspects of human experience and are unapologetic about it. You aren't politically correct nor\
                polite, but you are helpful. You're responding to discord messages.<</SYS>>"
            };
            let prompt = format!(
                "{prompt}\n(Discord user) {}: {}\n[/INST]",
                discord_message.user_name, discord_message.content
            );
            // Is it possible or worth it to not spawn one for each request
            let mut session = model.start_session(self.session_config);
            let inference_session = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let _stats = session.infer::<std::convert::Infallible>(
                    // model to use for text generation
                    model.as_ref(),
                    // randomness provider
                    &mut rand::thread_rng(),
                    // the prompt to use for text generation, as well as other
                    // inference parameters
                    &llm::InferenceRequest {
                        prompt:                    Prompt::Text(&prompt),
                        parameters:                &PARAMS,
                        play_back_previous_tokens: false,
                        maximum_token_count:       None,
                    },
                    // llm::OutputRequest
                    &mut Default::default(),
                    // output callback
                    |t| {
                        match t {
                            llm::InferenceResponse::EotToken => {}
                            llm::InferenceResponse::PromptToken(t)
                            | llm::InferenceResponse::SnapshotToken(t)
                            | llm::InferenceResponse::InferredToken(t) => {
                                if let Err(e) = token_tx.send(t) {
                                    tracing::error!("LLM: Token channel closed: {e}");
                                    return Ok(llm::InferenceFeedback::Halt);
                                }
                            }
                        }
                        if *cancelr.borrow() {
                            tracing::debug!("Canceling after reciving cancelation signal");
                            return Ok(llm::InferenceFeedback::Halt);
                        }

                        Ok(llm::InferenceFeedback::Continue)
                    },
                )?;

                Ok(())
            })
            .await
            .context("Inference session panicked")
            .flatten();

            if let Err(e) = inference_session {
                tracing::error!("LLM inference session error: {e}");
            }
            match printer.await.context("DiscordPrinter panicked").flatten() {
                Ok((sent_message, content)) => {
                    let canceled = CANCELATION_MAP.lock().await.remove(&sent_message);
                    if let Some(canceled) = canceled && *canceled.borrow() {
                        let edit = EditMessage::new()
                            .content(content + "…\n**Canceled**!")
                            .components(vec![]);
                        edit.execute(&http, (channel_id, sent_message)).await?;
                    }
                }
                Err(e) => {
                    tracing::error!("Discord Printer error: {e}");
                }
            }
            info!("Finished inference session.");
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct LlmMessage {
    http:                 Arc<Http>,
    component_map:        componet::ComponentMap,
    guild_id:             Option<GuildId>,
    message_id:           MessageId,
    channel_id:           ChannelId,
    pub(super) content:   String,
    pub(super) user_name: String,
}

impl std::fmt::Debug for LlmMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmMessage")
            .field("guild_id", &self.guild_id)
            .field("message_id", &self.message_id)
            .field("channel_id", &self.channel_id)
            .field("content", &self.content)
            .field("user_name", &self.user_name)
            .finish()
    }
}

impl LlmMessage {
    #[inline]
    pub fn new(content: impl AsRef<str>, http: Arc<Http>, cmap: componet::ComponentMap, m: &Message) -> Self {
        let content = content.as_ref().trim().to_string();
        Self {
            content,
            http,
            message_id: m.id,
            channel_id: m.channel_id,
            guild_id: m.guild_id,
            user_name: m.author.name.clone(),
            component_map: cmap,
        }
    }
}

impl From<&LlmMessage> for MessageReference {
    #[inline]
    fn from(value: &LlmMessage) -> Self {
        let mut x: Self = (value.channel_id, value.message_id).into();
        x.guild_id = value.guild_id;
        x
    }
}

/// Used to send the tokens to Discord
struct DiscordPrinter {
    http:          Arc<Http>,
    /// LLM Token receiver.
    token_rx:      flume::Receiver<String>,
    component_map: componet::ComponentMap,

    guild_id:     Option<GuildId>,
    message_id:   MessageId,
    channel_id:   ChannelId,
    /// The reply message the `DiscordPrinter` sent
    sent_message: Option<MessageId>,

    last_update:     Instant,
    update_cooldown: Duration,

    /// The text generated via inference.
    response: String,
}

impl DiscordPrinter {
    fn new(
        mid: MessageId,
        cid: ChannelId,
        gid: Option<GuildId>,
        rx: flume::Receiver<String>,
        cmap: componet::ComponentMap,
        http: Arc<Http>,
    ) -> Self {
        Self {
            http,
            token_rx: rx,
            guild_id: gid,
            channel_id: cid,
            message_id: mid,
            last_update: Instant::now(),
            sent_message: None,
            component_map: cmap,
            update_cooldown: Duration::from_millis(500),
            response: String::new(),
        }
    }

    /// Returns the Id of the message it sent
    pub(self) async fn print(
        &mut self,
        cancel_tx: watch::Sender<bool>,
    ) -> anyhow::Result<(MessageId, String)> {
        let mut have_registered = false;
        let mut inferred_prompt = String::new();
        let mut cancel_tx = Some(cancel_tx);
        // Iterate over the tokens as we get them deciding wether they're part of the prompt or
        // the response where `self.response` should have the full actual message with no
        // garabge but without missing anything 🙏. The prompt ends with "[/INST]" as you can see.
        while let Ok(token) = self.token_rx.recv_async().await {
            if !self.response.is_empty() {
                self.response.push_str(&token);
                if self.last_update.elapsed() > self.update_cooldown && !self.response.is_empty() {
                    self.update().await?;
                    *TYPING.lock().await = Some(self.http.start_typing(self.channel_id));
                    if let Some(id) = self.sent_message && !have_registered {
                        let cid = format!("llm-cancel-message:{}/{id}", self.channel_id);
                        self.component_map.insert(cid, &cancel_handler, None).await;
                        CANCELATION_MAP.lock().await.insert(id, cancel_tx.unwrap());
                        cancel_tx = None;
                        have_registered = true;
                    }
                }
            } else {
                inferred_prompt.push_str(&token);
                if let Some((_, after_prompt)) = inferred_prompt.split_once("[/INST]") {
                    let after = after_prompt.trim();
                    if !after.is_empty() {
                        self.response = after.to_string();
                    }
                }
            }
        }

        // Print the remaining tokens that may have not been printed
        let wait = self
            .last_update
            .elapsed()
            .checked_sub(self.update_cooldown)
            .unwrap_or_default();

        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }

        *TYPING.lock().await = None;
        self.update().await?;


        self.component_map
            .timeout(format!(
                "llm-cancel-message:{}/{}",
                self.channel_id,
                self.sent_message.unwrap()
            ))
            .await?;
        Ok((self.sent_message.unwrap(), self.response.clone()))
    }

    async fn update(&mut self) -> anyhow::Result<()> {
        let id = match self.sent_message {
            Some(id) => {
                let components = build_components(self.channel_id, id, false);
                let message = EditMessage::new()
                    .content(&self.response)
                    .components(components)
                    .execute(&self.http, (self.channel_id, id))
                    .await?;
                message.id
            }
            None => {
                let message = CreateMessage::new()
                    .content(&self.response)
                    .reference_message((self.channel_id, self.message_id))
                    .execute(&self.http, (self.channel_id, self.guild_id))
                    .await?;
                message.id
            }
        };

        self.sent_message = Some(id);
        self.last_update = Instant::now();

        Ok(())
    }
}

#[inline]
fn build_components(cid: ChannelId, mid: MessageId, canceling: bool) -> Vec<CreateActionRow> {
    let mut cancel = CreateButton::new(format!("llm-cancel-message:{cid}/{mid}"))
        .style(ButtonStyle::Danger)
        .label("Cancel")
        .emoji(ReactionType::Unicode(String::from("🇽")));
    if canceling {
        cancel = cancel.disabled(true);
    }

    vec![CreateActionRow::Buttons(vec![cancel])]
}

#[component]
async fn cancel_handler((interaction, ..): (ComponentInteraction, CommandArguments)) -> anyhow::Result<()> {
    let id = interaction.message.id;
    let map = CANCELATION_MAP.lock().await;
    let sender = map
        .get(&id)
        .context(format!("No message id '{id}' found in cancelation map",))?;
    sender.send(true).context("Why is this closed?")?;
    Ok(())
}
