// SMTP stub — lettre is not vendored; email sending is a no-op in dev builds.
#[allow(dead_code)]
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
        _to_email: &str,
        _subject: &str,
        _body: &str,
    ) -> Result<(), String> {
        Err("SMTP not available: lettre not vendored in this build".to_string())
    }
}
