use chrono::Local;
use directories::ProjectDirs;
use dotenvy::dotenv;
use iced::font::{Family, Stretch, Style, Weight};
use iced::widget::button::background;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Border, Element, Length, Padding, Task, Theme};
use image;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

mod ai;
mod mail;

const BODY_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Pretendard Variable"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ActiveButton {
    Previous,
    Current,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tits {
    summary: String,
    last_updated: String,
    previous_briefing: Option<String>,
    current_briefing: Option<String>,
    previous_update: Option<String>,
    update_time: Option<String>,
    active: ActiveButton,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshPressed,
    SummaryGenerated(Result<String, String>),
    PreviousBriefing,
    CurrentBriefing,
}

impl Default for Tits {
    fn default() -> Self {
        Self {
            summary: String::from(
                "You have a quiet morning. \n\n\
                 There are no urgent blockers in your inbox. \n\n",
            ),
            last_updated: String::from("Last updated: Just now"),
            previous_briefing: None,
            current_briefing: None,
            previous_update: None,
            update_time: None,
            active: ActiveButton::Current,
        }
    }
}

impl Tits {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshPressed => {
                self.previous_briefing = Some(self.summary.clone());

                let now = Local::now();
                let formatted = now.format("%b %-d, %-I:%M %p").to_string();
                self.previous_update = self.update_time.take();
                self.update_time = Some(formatted);

                self.last_updated = String::from("Refreshing...");
                self.summary = String::from("Reading inbox...");

                self.save();

                Task::perform(refresh_inbox(), Message::SummaryGenerated)
            }

            Message::SummaryGenerated(result) => {
                match result {
                    Ok(text) => {
                        self.summary = text;
                        self.current_briefing = Some(self.summary.clone());
                        self.last_updated = String::from("Updated: Just now");
                    }
                    Err(error) => {
                        self.summary = format!("Error: {}", error);
                        self.current_briefing = Some(self.summary.clone());
                        self.last_updated = String::from("Error");
                    }
                }

                self.active = ActiveButton::Current;
                self.save();

                Task::none()
            }

            Message::PreviousBriefing => {
                self.summary = self.previous_briefing.clone().unwrap_or_default();

                let last = self
                    .previous_update
                    .clone()
                    .unwrap_or_else(|| "Forever ago".to_string());
                self.last_updated = format!("Last Updated at: {}", last);

                self.active = ActiveButton::Previous;
                self.save();

                Task::none()
            }

            Message::CurrentBriefing => {
                self.summary = self.current_briefing.clone().unwrap_or_default();

                let last = self
                    .update_time
                    .clone()
                    .unwrap_or_else(|| "Forever ago".to_string());
                self.last_updated = format!("Last Updated at: {}", last);

                self.active = ActiveButton::Current;
                self.save();

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let btn_previous = match (self.active.clone(), self.previous_briefing.is_some()) {
            (ActiveButton::Previous, _) => button("<"),
            (_, true) => button("<").on_press(Message::PreviousBriefing),
            _ => button("<"),
        };
        let btn_next = match (self.active.clone(), self.current_briefing.is_some()) {
            (ActiveButton::Current, _) => button(">"),
            (_, true) => button(">").on_press(Message::CurrentBriefing),
            _ => button(">"),
        };

        let mut content = column![
            scrollable(
                column![text(&self.summary).font(BODY_FONT)].padding(Padding {
                    top: 80.0,
                    right: 40.0,
                    bottom: 40.0,
                    left: 80.0,
                })
            )
            .height(Length::Fill),
            text(&self.last_updated)
                .font(BODY_FONT)
                .size(14)
                .color(iced::Color::from_rgb8(200, 200, 200)),
            button(
                text("⭮ Refresh")
                    .font(BODY_FONT)
                    .size(12)
                    .color(iced::Color::from_rgb8(156, 156, 156))
            )
            .on_press(Message::RefreshPressed)
            .style(|_theme, _state| {
                button::Style {
                    background: Some(iced::Color::from_rgb8(30, 30, 30).into()),
                    ..Default::default()
                }
            }),
            row![
                btn_previous
                    .style(|_theme, _state| {
                        button::Style {
                            border: Border {
                                radius: iced::border::Radius {
                                    top_left: 6.0,
                                    top_right: 0.0,
                                    bottom_left: 6.0,
                                    bottom_right: 0.0,
                                },
                                width: 0.5,
                                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                                ..Default::default()
                            },
                            background: Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
                            text_color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5),
                            ..Default::default()
                        }
                    })
                    .padding(iced::Padding::from([4.0, 24.0])),
                btn_next
                    .style(|_theme, _state| {
                        button::Style {
                            border: Border {
                                radius: iced::border::Radius {
                                    top_left: 0.0,
                                    top_right: 6.0,
                                    bottom_left: 0.0,
                                    bottom_right: 6.0,
                                },
                                width: 0.5,
                                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                                ..Default::default()
                            },
                            background: Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
                            text_color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.5),
                            ..Default::default()
                        }
                    })
                    .padding(iced::Padding::from([4.0, 24.0]))
            ]
        ]
        .max_width(800)
        .spacing(10)
        .align_x(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(120)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb8(30, 30, 30).into()),
                border: Border {
                    radius: iced::border::Radius::from(6.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn get_state_file() -> PathBuf {
        let project_dirs = ProjectDirs::from("com", "Apex", "tit-babbler")
            .expect("Could not determine project directory");

        let config_dir = project_dirs.config_dir();

        if !config_dir.exists() {
            fs::create_dir_all(config_dir).expect("Failed to create config directory");
        }

        config_dir.join("state.json")
    }

    fn save(&self) {
        let path = Self::get_state_file();
        let json = serde_json::to_string(self).expect("Failed to serialize state");

        fs::write(path, json).expect("Failed to write state file");
    }

    fn load() -> Self {
        let path = Self::get_state_file();

        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }

        Self::default()
    }
}

pub async fn refresh_inbox() -> Result<String, String> {
    let emails = mail::fetch_emails().await?;
    let formatted_emails = mail::email_formatter(emails);

    if formatted_emails.is_empty() {
        return Ok(String::new());
    }

    let response = ai::generate_response(format!(
        r#"<system_capability>
    You are an elite Executive Assistant and Chief of Staff. Your goal is to synthesize high-volume information into calm, actionable intelligence. You value clarity, brevity, and narrative flow over lists and formatting.
    </system_capability>

    <strict_authority_protocol>
    ### FORMATTING CONSTANTS - READ CAREFULLY
    1.  **PLAIN TEXT ONLY**: You are STRICTLY FORBIDDEN from using Markdown.
        -   NO bolding (**text**).
        -   NO italics (*text*).
        -   NO headers (###).
        -   NO bullet points (-) or numbered lists (1.).
    2.  **PARAGRAPHS**: Content must be delivered in smooth, readable paragraphs.
    3.  **FAILURE CONDITION**: If the output contains a single asterisk or bullet point, the response is considered a failure.
    </strict_authority_protocol>

    <processing_logic>
    Step 1: **FILTER**. Aggressively discard trivial emails (newsletters, receipts, notifications, "checking in" emails) unless they contain a direct blocker or urgent deadline - USERS DO NOT WANT SPAM IN THEIR BREIFING.
    Step 2: **EXTRACT**. Identify:
        -   Upcoming meetings (Who, When, Context).
        -   Direct questions asked of the user.
        -   Urgent blockers or red flags.
        -   Status updates on active projects.
    Step 3: **SYNTHESIZE**. Draft a briefing using a calm, professional tone.
        -   Start with "Good day, Apex.".
        -   Group related items into paragraphs (e.g., Meeting context in para 1, Project blockers in para 2).
        -   End with a strategic next step if applicable.
    </processing_logic>

    <few_shot_examples>
    Input: [Raw Emails containing: 1. Newsletter from Substack, 2. Meeting reminder for ScyAI at 7pm, 3. Email from Bernhard about missing login screen, 4. WhatsApp group chatter about QR codes vs Roam, 5. SuperWhisper team update on landing page.]

    Output:
    Good day, Apex.

    You have a meeting coming up in about 3.5 hours - ScyAI x UI/UX Sync at 7pm with Bernhard. Before that call, you should know that Bernhard flagged a missing login screen in the ScyAI Design group. They're implementing one-time passwords for first login, but users need to change their password immediately after. He's looking for that additional screen to be designed.

    Also in your WhatsApp groups, someone from the Visualizations/Branding Co is asking about QR codes and whether you prefer communication through that chat or Roam. Julian pushed back hard on QR codes, but the original question about your preferred communication method is still hanging.

    Your SuperWhisper team has been busy - they've got a new landing page ready for feedback. The conversation shows they've been iterating on animations and user experience, with some good discussion about making the demo less interactive during autoplay.

    I'd prioritize prepping for the ScyAI meeting by reviewing that missing login screen requirement. The day looks manageable with just the one evening meeting.
    </few_shot_examples>

    <task>
    Summarize the following raw emails into a morning briefing following the strict formatting protocols above.

    EMAILS:
    {}
    </task>"#,
        formatted_emails
    ))
    .await?;

    Ok(response)
}

fn load_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/icon.png");

    let image = image::load_from_memory(bytes).ok()?.to_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    iced::window::icon::from_rgba(rgba, width, height).ok()
}

pub fn main() -> iced::Result {
    dotenv().ok();
    println!("Key found!");

    iced::application(Tits::load, Tits::update, Tits::view)
        .title(|_: &Tits| String::from("Tit-Babbler"))
        .theme(|_: &Tits| Theme::Dark)
        .window(iced::window::Settings {
            decorations: true,
            transparent: false,
            icon: load_icon(),
            ..Default::default()
        })
        .run()
}
