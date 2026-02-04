use chrono::Local;
use directories::ProjectDirs;
use dotenvy::dotenv;
use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Length, Task, Theme};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

mod ai;
mod mail;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tits {
    summary: String,
    last_updated: String,
    previous_briefing: Option<String>,
    previous_update: Option<String>,
    update_time: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshPressed,
    SummaryGenerated(Result<String, String>),
    PreviousBriefing,
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
            previous_update: None,
            update_time: None,
        }
    }
}

impl Tits {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshPressed => {
                self.last_updated = String::from("Refreshing...");
                self.summary = String::from("Reading inbox...");
                self.previous_briefing = Some(self.summary.clone());

                let now = Local::now();
                let formatted = now.format("%b %-d, %-I:%M %p").to_string();
                self.previous_update = self.update_time.take();
                self.update_time = Some(formatted);

                self.save();

                Task::perform(refresh_inbox(), Message::SummaryGenerated)
            }

            Message::SummaryGenerated(result) => {
                match result {
                    Ok(text) => {
                        self.summary = text;
                        self.last_updated = String::from("Updated: Just now");
                        self.save();
                    }
                    Err(error) => {
                        self.summary = format!("Error: {}", error);
                        self.last_updated = String::from("Error");
                        self.save();
                    }
                }

                Task::none()
            }

            Message::PreviousBriefing => {
                self.summary = self.previous_briefing.clone().unwrap_or_default();

                let last = self
                    .previous_update
                    .clone()
                    .unwrap_or_else(|| "Forever ago".to_string());
                self.last_updated = format!("Last Updated at: {}", last);

                self.save();

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let mut content = column![
            text(&self.summary),
            button("Refresh").on_press(Message::RefreshPressed),
        ]
        .max_width(650)
        .spacing(20);

        if self.previous_briefing.is_some() {
            content =
                content.push(button("Show Previous Briefing").on_press(Message::PreviousBriefing))
        }

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
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

    let response =
        ai::generate_response(format!("Summarise these emails:\n{}", formatted_emails)).await?;

    Ok(response)
}

pub fn main() -> iced::Result {
    dotenv().ok();
    println!("Key found!");

    iced::application(Tits::load, Tits::update, Tits::view)
        .title(|_: &Tits| String::from("Tit-Babbler"))
        .theme(|_: &Tits| Theme::Dark)
        .run()
}
