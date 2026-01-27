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
}

#[derive(Debug, Clone)]
enum Message {
    RefreshPressed,
    SummaryGenerated(Result<String, String>),
}

impl Default for Tits {
    fn default() -> Self {
        Self {
            summary: String::from(
                "You have a quiet morning. \n\n\
                 There are no urgent blockers in your inbox. \n\n",
            ),
            last_updated: String::from("Last updated: forever ago"),
        }
    }
}

impl Tits {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshPressed => {
                self.last_updated = String::from("Refreshing...");
                self.summary = String::from("Reading inbox...");

                let fake_inbox = r#"Email 1: From Boss. Subject: Urgent presentation. Body: We need the slides by 2pm.
    Email 2: From Amazon. Subject: Order Shipped. Body: Your socks are on the way.
    Email 3: From David. Subject: Lunch? Body: Tacos at 12?
    Email 4: From Jira. Subject: Ticket #994. Body: Update on the backend bug."#;

                self.save();

                Task::perform(
                    ai::generate_response(format!(
                        "Summarise these email and produce a briefing: {}",
                        fake_inbox
                    )),
                    Message::SummaryGenerated,
                )
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
        }
    }

    fn view(&self) -> Element<Message> {
        let content = column![
            text(&self.summary),
            button("Refresh").on_press(Message::RefreshPressed)
        ]
        .max_width(650)
        .spacing(20);

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

pub fn main() -> iced::Result {
    dotenv().ok();
    println!("Key found!");

    iced::application(Tits::load, Tits::update, Tits::view)
        .title(|_: &Tits| String::from("Tit-Babbler"))
        .theme(|_: &Tits| Theme::Dark)
        .run()
}
