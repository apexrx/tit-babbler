use chrono::{Duration, Local};
use futures::stream::StreamExt;
use rustls::pki_types::ServerName;
use std::env;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

// Helper
type Tls = TlsStream<TcpStream>;

pub struct Email {
    pub subject: String,
    pub body: String,
    pub from: String,
}

fn get_header_value(parsed: &mailparse::ParsedMail, name: &str) -> Option<String> {
    parsed
        .headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case(name))
        .map(|h| h.get_value())
}

fn extract_body(parsed: &mailparse::ParsedMail) -> Result<String, mailparse::MailParseError> {
    // If this part is text/plain or text/html, use it directly
    if parsed.ctype.mimetype.starts_with("text/") {
        return parsed.get_body();
    }

    // Otherwise, walk subparts (multipart/*)
    for subpart in &parsed.subparts {
        if subpart.ctype.mimetype == "text/plain" {
            return subpart.get_body();
        }
    }

    // Fallback: try first subpart with any text/*
    for subpart in &parsed.subparts {
        if subpart.ctype.mimetype.starts_with("text/") {
            return subpart.get_body();
        }
    }

    // Last resort
    parsed.get_body()
}

pub async fn fetch_emails() -> Result<Vec<Email>, String> {
    let imap_server = env::var("IMAP_SERVER").expect("IMAP_SERVER not set");
    let imap_username = env::var("IMAP_USERNAME").expect("IMAP_USERNAME not set");
    let imap_password = env::var("IMAP_PASSWORD").expect("IMAP_PASSWORD not set");

    // Establishing a connection
    let tcp = TcpStream::connect((imap_server.as_str(), 993u16))
        .await
        .map_err(|e| format!("Failed to connect to IMAP server: {}", e))?;

    // Certificate store, config build & connector
    let root_store = RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS
            .iter()
            .map(|ta| ta.to_owned()),
    );

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    // Converting server name
    let domain =
        ServerName::try_from(imap_server.to_owned()).map_err(|_| "Invalid DNS name".to_string())?;

    let tls = connector
        .connect(domain, tcp)
        .await
        .map_err(|e| format!("Failed to establish TLS connection: {}", e))?;

    // Wrap stream and login
    let mut imap: async_imap::Session<Tls> = async_imap::Client::new(tls)
        .login(imap_username, imap_password)
        .await
        .map_err(|(e, _)| format!("Failed to login to IMAP server: {}", e))?;

    // Selecting inbox and mails from yesterday
    let yesterday = Local::now()
        .checked_sub_signed(Duration::days(1))
        .unwrap()
        .format("%d-%b-%Y")
        .to_string();
    let inbox = imap
        .select("INBOX")
        .await
        .map_err(|e| format!("Failed to select inbox: {}", e))?;

    // Searching the inbox
    let search_query = format!("SINCE {}", yesterday);
    let mails = imap
        .search(&search_query)
        .await
        .map_err(|e| format!("Failed to search inbox: {}", e))?;
    if mails.len() == 0 {
        return Ok(Vec::new());
    }

    // Fetching and parsing mails
    let sequence_set = mails
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let mut stream = imap
        .fetch(&sequence_set, "RFC822")
        .await
        .map_err(|e| format!("Failed to fetch emails: {}", e))?;

    let mut fetch_emails = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => {
                let email_body = message.body().unwrap();
                let parsed = mailparse::parse_mail(email_body)
                    .map_err(|e| format!("Failed to parse mail: {}", e))?;

                let subject = get_header_value(&parsed, "Subject")
                    .unwrap_or_else(|| "(No Subject)".to_string());
                let from = get_header_value(&parsed, "From")
                    .unwrap_or_else(|| "(Unknown Sender)".to_string());
                let body = extract_body(&parsed).unwrap_or_else(|_| "(No Body)".to_string());

                fetch_emails.push(Email {
                    subject,
                    body,
                    from,
                });
            }
            Err(e) => eprintln!("Error fetching a message: {}", e),
        }
    }

    Ok(fetch_emails)
}

pub fn email_formatter(emails: Vec<Email>) -> String {
    if emails.is_empty() {
        return String::new();
    }

    let formatted_emails = emails
        .iter()
        .map(|email| {
            format!(
                "Subject: {}\nFrom: {}\nBody: {}\n",
                email.subject, email.from, email.body
            )
        })
        .collect::<Vec<String>>()
        .join("-----------\n");

    formatted_emails
}
