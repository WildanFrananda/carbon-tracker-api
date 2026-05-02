use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::error::Error;
use std::env;

pub fn send_reset_email(to_email: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let smtp_server = env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string());
    let smtp_user = env::var("SMTP_USER").expect("SMTP_USER must be set");
    let smtp_pass = env::var("SMTP_PASS").expect("SMTP_PASS must be set");

    let base_url = env::var("APP_DEEP_LINK_URL").unwrap_or_else(|_| "carbonfootprinttracker://".to_string());
    let reset_link = format!("{}reset-password?token={}", base_url, token);

    let email = Message::builder()
        .from(format!("Carbon Tracker <{}>", smtp_user).parse()?)
        .to(to_email.parse()?)
        .subject("Reset Password Anda")
        .body(format!(
            "Halo,\n\nKami menerima permintaan untuk reset password akun Carbon Tracker Anda.\nKlik link berikut untuk reset password: {}\n\nLink ini akan kadaluarsa dalam 1 jam.\n\nJika Anda tidak merasa melakukan permintaan ini, abaikan email ini.",
            reset_link
        ))?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&smtp_server)?
        .credentials(creds)
        .build();

    mailer.send(&email)?;
    return Ok(());
}