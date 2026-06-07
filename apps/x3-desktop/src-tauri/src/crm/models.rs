use serde::{Deserialize, Serialize};

/* ── Contact ─────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub owner_user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub job_title: String,
    pub avatar_url: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
    pub website: String,
    pub notes: String,
    pub tags: String,
    pub source: String,
    pub stage: String,
    pub priority: String,
    pub last_contacted: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactInput {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub source: Option<String>,
    pub stage: Option<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub avatar_url: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub source: Option<String>,
    pub stage: Option<String>,
    pub priority: Option<String>,
}

/* ── Calendar Event ──────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub owner_user_id: String,
    pub title: String,
    pub description: String,
    pub location: String,
    pub event_type: String,
    pub start_at: String,
    pub end_at: String,
    pub all_day: bool,
    pub color: String,
    pub recurrence: String,
    pub reminder_mins: i32,
    pub contact_id: String,
    pub deal_id: String,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventInput {
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub event_type: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub all_day: Option<bool>,
    pub color: Option<String>,
    pub recurrence: Option<String>,
    pub reminder_mins: Option<i32>,
    pub contact_id: Option<String>,
    pub deal_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub event_type: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub all_day: Option<bool>,
    pub color: Option<String>,
    pub recurrence: Option<String>,
    pub reminder_mins: Option<i32>,
    pub contact_id: Option<String>,
    pub deal_id: Option<String>,
    pub completed: Option<bool>,
}

/* ── Deal ────────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deal {
    pub id: String,
    pub owner_user_id: String,
    pub contact_id: String,
    pub title: String,
    pub value: f64,
    pub currency: String,
    pub stage: String,
    pub probability: i32,
    pub expected_close: String,
    pub notes: String,
    pub won: bool,
    pub lost: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDealInput {
    pub contact_id: Option<String>,
    pub title: String,
    pub value: Option<f64>,
    pub currency: Option<String>,
    pub stage: Option<String>,
    pub probability: Option<i32>,
    pub expected_close: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDealInput {
    pub contact_id: Option<String>,
    pub title: Option<String>,
    pub value: Option<f64>,
    pub currency: Option<String>,
    pub stage: Option<String>,
    pub probability: Option<i32>,
    pub expected_close: Option<String>,
    pub notes: Option<String>,
    pub won: Option<bool>,
    pub lost: Option<bool>,
}

/* ── Activity ────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub owner_user_id: String,
    pub contact_id: String,
    pub deal_id: String,
    pub event_id: String,
    pub activity_type: String,
    pub subject: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivityInput {
    pub contact_id: Option<String>,
    pub deal_id: Option<String>,
    pub event_id: Option<String>,
    pub activity_type: String,
    pub subject: Option<String>,
    pub body: Option<String>,
}

/* ── Email Template ──────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplate {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub subject: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmailTemplateInput {
    pub name: String,
    pub subject: String,
    pub body: String,
}

/* ── SMTP Config ─────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub id: String,
    pub owner_user_id: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub from_name: String,
    pub from_email: String,
    pub use_tls: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSmtpConfigInput {
    pub host: String,
    pub port: Option<i32>,
    pub username: String,
    pub password: String,
    pub from_name: String,
    pub from_email: String,
    pub use_tls: Option<bool>,
}

/* ── Send Email ──────────────────────────────────── */
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailInput {
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub contact_id: Option<String>,
    pub template_id: Option<String>,
}

/* ── Sent Email ──────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentEmail {
    pub id: String,
    pub owner_user_id: String,
    pub contact_id: String,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub error_message: String,
    pub template_id: String,
    pub created_at: String,
}

/* ── CRM Stats ───────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrmStats {
    pub contact_count: i32,
    pub deal_count: i32,
    pub open_deal_value: f64,
    pub won_deal_count: i32,
    pub event_count: i32,
    pub upcoming_events: i32,
    pub email_sent_count: i32,
    pub activity_count: i32,
}

/* ── CSV Import ──────────────────────────────────── */
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportRequest {
    pub csv_content: String,
    pub column_mapping: std::collections::HashMap<String, String>, // CSV header -> Contact field
    pub skip_duplicates: bool,
    pub update_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub imported_count: i32,
    pub duplicate_count: i32,
    pub updated_count: i32,
    pub error_count: i32,
    pub errors: Vec<String>,
}

/* ── Contact Deduplication ───────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateContact {
    pub id1: String,
    pub id2: String,
    pub name1: String,
    pub name2: String,
    pub email1: String,
    pub email2: String,
    pub similarity_score: f32,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeContactsInput {
    pub primary_id: String,
    pub secondary_id: String,
    pub keep_fields: std::collections::HashMap<String, String>, // Field -> which ID's value to keep
}

/* ── Campaign Management ─────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Campaign {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub description: String,
    pub campaign_type: String, // email, sms, social, etc.
    pub status: String,        // draft, scheduled, active, completed
    pub target_contacts: i32,
    pub sent_count: i32,
    pub opened_count: i32,
    pub clicked_count: i32,
    pub conversion_count: i32,
    pub scheduled_at: String,
    pub started_at: String,
    pub completed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCampaignInput {
    pub name: String,
    pub description: Option<String>,
    pub campaign_type: String,
    pub scheduled_at: Option<String>,
}

/* ── Lead Scoring ────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadScore {
    pub contact_id: String,
    pub score: i32,      // 0-100
    pub grade: String,   // A, B, C, D, F
    pub engagement_points: i32,
    pub company_points: i32,
    pub behavioral_points: i32,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactWithScore {
    pub contact: Contact,
    pub lead_score: LeadScore,
}

/* ── Bulk Actions ────────────────────────────────── */
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkUpdateInput {
    pub contact_ids: Vec<String>,
    pub updates: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkActionResult {
    pub success_count: i32,
    pub failure_count: i32,
    pub errors: Vec<String>,
}

/* ── Deal Forecasting ────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DealForecast {
    pub month: String,  // YYYY-MM
    pub confidence_low: f64,
    pub confidence_mid: f64,
    pub confidence_high: f64,
    pub expected_value: f64,
    pub historical_accuracy: f32, // 0-100 %
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineAnalytics {
    pub total_value: f64,
    pub total_deals: i32,
    pub average_deal_value: f64,
    pub weighted_forecast: f64,
    pub stage_breakdown: std::collections::HashMap<String, PipelineStageStats>,
    pub months_forecast: Vec<DealForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageStats {
    pub stage_name: String,
    pub count: i32,
    pub total_value: f64,
    pub avg_days_in_stage: f32,
    pub win_probability: f32,
}

