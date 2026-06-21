/// SMTP email sender — disabled when lettre is not vendored.
/// Sending will log a warning and return an error at runtime.
pub struct SmtpSender {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_name: String,
    pub from_email: String,
    pub use_tls: bool,
}

impl SmtpSender {
    pub async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        eprintln!(
            "[smtp] email suppressed: to={to_email}, subject={subject}, body_len={}",
            body.len()
        );
        Err(format!(
            "SMTP disabled: lettre is not vendored. Would have sent to {to_email} with subject \"{subject}\""
        ))
    }
}
