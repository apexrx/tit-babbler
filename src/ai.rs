use serde::{Deserialize, Serialize};
use std::env;

// Structure for request and response

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Part {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<Content>,
}

impl GeminiRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.into(),
                }],
            }],
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
pub struct Candidate {
    pub content: Content,
}

impl GeminiResponse {
    pub fn first_text(&self) -> Option<String> {
        self.candidates.first().map(|c| {
            c.content
                .parts
                .iter()
                .map(|p| p.text.as_str())
                .collect::<String>()
        })
    }
}

pub async fn generate_response(prompt: String) -> Result<String, String> {
    let api_key =
        env::var("GEMINI_API_KEY").map_err(|e| format!("Failed to get GEMINI_API_KEY: {}", e))?;
    let client = reqwest::Client::new();
    let request = GeminiRequest::new(prompt);

    let response = client.post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Check if the response is successful
    if !response.status().is_success() {
        Err(String::from("Failed to parse response"))
    } else {
        let response = response
            .json::<GeminiResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        response.first_text().ok_or("No text generated".to_string())
    }
}
