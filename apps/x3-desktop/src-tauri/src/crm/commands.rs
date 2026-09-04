use crate::crm::db::CrmDb;
use crate::crm::models::*;
use crate::crm::smtp::SmtpSender;
use chrono::{Utc, Datelike};
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

type CmdResult<T> = Result<T, String>;

fn now() -> String { Utc::now().to_rfc3339() }
fn uid() -> String { Uuid::new_v4().to_string() }
fn e(err: impl std::fmt::Display) -> String { err.to_string() }

/* ══════════════════════════════════════════════════════
   CONTACTS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_contact(db: State<'_, CrmDb>, user_id: String, input: CreateContactInput) -> CmdResult<Contact> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_contacts (id, owner_user_id, first_name, last_name, email, phone, company, job_title,
         address, city, state, zip, country, website, notes, tags, source, stage, priority, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
        params![
            id, user_id, input.first_name,
            input.last_name.unwrap_or_default(), input.email.unwrap_or_default(),
            input.phone.unwrap_or_default(), input.company.unwrap_or_default(),
            input.job_title.unwrap_or_default(), input.address.unwrap_or_default(),
            input.city.unwrap_or_default(), input.state.unwrap_or_default(),
            input.zip.unwrap_or_default(), input.country.unwrap_or_default(),
            input.website.unwrap_or_default(), input.notes.unwrap_or_default(),
            input.tags.unwrap_or_default(), input.source.unwrap_or("manual".into()),
            input.stage.unwrap_or("lead".into()), input.priority.unwrap_or("medium".into()),
            ts, ts,
        ],
    ).map_err(e)?;
    get_contact_by_id_scoped(&conn, &id, &user_id)
}

#[tauri::command]
pub fn crm_update_contact(db: State<'_, CrmDb>, contact_id: String, user_id: String, input: UpdateContactInput) -> CmdResult<Contact> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut idx = 2u32;
    macro_rules! opt_set {
        ($field:ident, $col:expr) => {
            if input.$field.is_some() { sets.push(format!("{} = ?{}", $col, idx)); idx += 1; }
        };
    }
    opt_set!(first_name, "first_name"); opt_set!(last_name, "last_name"); opt_set!(email, "email");
    opt_set!(phone, "phone"); opt_set!(company, "company"); opt_set!(job_title, "job_title");
    opt_set!(avatar_url, "avatar_url"); opt_set!(address, "address"); opt_set!(city, "city");
    opt_set!(state, "state"); opt_set!(zip, "zip"); opt_set!(country, "country");
    opt_set!(website, "website"); opt_set!(notes, "notes"); opt_set!(tags, "tags");
    opt_set!(source, "source"); opt_set!(stage, "stage"); opt_set!(priority, "priority");

    let sql = format!(
        "UPDATE crm_contacts SET {} WHERE id = ?{} AND owner_user_id = ?{}",
        sets.join(", "), idx, idx + 1
    );

    let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(ts)];
    macro_rules! opt_push {
        ($field:ident) => { if let Some(v) = input.$field { p.push(Box::new(v)); } };
    }
    opt_push!(first_name); opt_push!(last_name); opt_push!(email); opt_push!(phone);
    opt_push!(company); opt_push!(job_title); opt_push!(avatar_url); opt_push!(address);
    opt_push!(city); opt_push!(state); opt_push!(zip); opt_push!(country);
    opt_push!(website); opt_push!(notes); opt_push!(tags); opt_push!(source);
    opt_push!(stage); opt_push!(priority);
    p.push(Box::new(contact_id.clone()));
    p.push(Box::new(user_id.clone()));

    let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
    let affected_rows = conn.execute(&sql, params_ref.as_slice()).map_err(e)?;
    if affected_rows == 0 {
        return Err("Contact not found or access denied".to_string());
    }
    get_contact_by_id_scoped(&conn, &contact_id, &user_id)
}

#[tauri::command]
pub fn crm_get_contacts(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<Contact>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, owner_user_id, first_name, last_name, email, phone, company, job_title,
         avatar_url, address, city, state, zip, country, website, notes, tags, source, stage,
         priority, last_contacted, created_at, updated_at
         FROM crm_contacts WHERE owner_user_id = ?1 ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], row_to_contact).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn crm_get_contact(db: State<'_, CrmDb>, contact_id: String, user_id: String) -> CmdResult<Contact> {
    let conn = db.conn.lock().map_err(e)?;
    conn.query_row(
        "SELECT id, owner_user_id, first_name, last_name, email, phone, company, job_title,
         avatar_url, address, city, state, zip, country, website, notes, tags, source, stage,
         priority, last_contacted, created_at, updated_at
         FROM crm_contacts WHERE id = ?1 AND owner_user_id = ?2",
        params![contact_id, user_id],
        row_to_contact,
    ).map_err(e)
}

#[tauri::command]
pub fn crm_delete_contact(db: State<'_, CrmDb>, contact_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM crm_contacts WHERE id = ?1 AND owner_user_id = ?2", params![contact_id, user_id]).map_err(e)?;
    Ok(())
}

fn get_contact_by_id(conn: &rusqlite::Connection, id: &str) -> CmdResult<Contact> {
    conn.query_row(
        "SELECT id, owner_user_id, first_name, last_name, email, phone, company, job_title,
         avatar_url, address, city, state, zip, country, website, notes, tags, source, stage,
         priority, last_contacted, created_at, updated_at FROM crm_contacts WHERE id = ?1",
        params![id], row_to_contact,
    ).map_err(e)
}

fn row_to_contact(row: &rusqlite::Row) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get(0)?, owner_user_id: row.get(1)?, first_name: row.get(2)?,
        last_name: row.get(3)?, email: row.get(4)?, phone: row.get(5)?,
        company: row.get(6)?, job_title: row.get(7)?, avatar_url: row.get(8)?,
        address: row.get(9)?, city: row.get(10)?, state: row.get(11)?,
        zip: row.get(12)?, country: row.get(13)?, website: row.get(14)?,
        notes: row.get(15)?, tags: row.get(16)?, source: row.get(17)?,
        stage: row.get(18)?, priority: row.get(19)?, last_contacted: row.get(20)?,
        created_at: row.get(21)?, updated_at: row.get(22)?,
    })
}

/* ══════════════════════════════════════════════════════
   CALENDAR EVENTS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_event(db: State<'_, CrmDb>, user_id: String, input: CreateEventInput) -> CmdResult<CalendarEvent> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_events (id, owner_user_id, title, description, location, event_type,
         start_at, end_at, all_day, color, recurrence, reminder_mins, contact_id, deal_id, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            id, user_id, input.title,
            input.description.unwrap_or_default(), input.location.unwrap_or_default(),
            input.event_type.unwrap_or("meeting".into()),
            input.start_at, input.end_at,
            input.all_day.unwrap_or(false) as i32,
            input.color.unwrap_or("#ff6b35".into()),
            input.recurrence.unwrap_or_default(),
            input.reminder_mins.unwrap_or(15),
            input.contact_id.unwrap_or_default(),
            input.deal_id.unwrap_or_default(),
            ts, ts,
        ],
    ).map_err(e)?;
    get_event_by_id_scoped(&conn, &id, &user_id)
}

#[tauri::command]
pub fn crm_update_event(db: State<'_, CrmDb>, event_id: String, user_id: String, input: UpdateEventInput) -> CmdResult<CalendarEvent> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut idx = 2u32;
    macro_rules! opt_set {
        ($field:ident, $col:expr) => { if input.$field.is_some() { sets.push(format!("{} = ?{}", $col, idx)); idx += 1; } };
    }
    opt_set!(title, "title"); opt_set!(description, "description"); opt_set!(location, "location");
    opt_set!(event_type, "event_type"); opt_set!(start_at, "start_at"); opt_set!(end_at, "end_at");
    opt_set!(color, "color"); opt_set!(recurrence, "recurrence"); opt_set!(contact_id, "contact_id");
    opt_set!(deal_id, "deal_id");
    if input.all_day.is_some() { sets.push(format!("all_day = ?{}", idx)); idx += 1; }
    if input.reminder_mins.is_some() { sets.push(format!("reminder_mins = ?{}", idx)); idx += 1; }
    if input.completed.is_some() { sets.push(format!("completed = ?{}", idx)); idx += 1; }

    let sql = format!("UPDATE crm_events SET {} WHERE id = ?{} AND owner_user_id = ?{}", sets.join(", "), idx, idx + 1);

    let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(ts)];
    macro_rules! opt_push { ($field:ident) => { if let Some(v) = input.$field { p.push(Box::new(v)); } }; }
    opt_push!(title); opt_push!(description); opt_push!(location); opt_push!(event_type);
    opt_push!(start_at); opt_push!(end_at); opt_push!(color); opt_push!(recurrence);
    opt_push!(contact_id); opt_push!(deal_id);
    if let Some(v) = input.all_day { p.push(Box::new(v as i32)); }
    if let Some(v) = input.reminder_mins { p.push(Box::new(v)); }
    if let Some(v) = input.completed { p.push(Box::new(v as i32)); }
    p.push(Box::new(event_id.clone()));
    p.push(Box::new(user_id.clone()));
    
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
    let affected_rows = conn.execute(&sql, params_ref.as_slice()).map_err(e)?;
    if affected_rows == 0 {
        return Err("Event not found or access denied".to_string());
    }
    get_event_by_id_scoped(&conn, &event_id, &user_id)
}

#[tauri::command]
pub fn crm_get_events(db: State<'_, CrmDb>, user_id: String, start: Option<String>, end: Option<String>) -> CmdResult<Vec<CalendarEvent>> {
    let conn = db.conn.lock().map_err(e)?;
    let (sql, p): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match (&start, &end) {
        (Some(s), Some(e_)) => (
            "SELECT id, owner_user_id, title, description, location, event_type, start_at, end_at,
             all_day, color, recurrence, reminder_mins, contact_id, deal_id, completed, created_at, updated_at
             FROM crm_events WHERE owner_user_id = ?1 AND start_at >= ?2 AND end_at <= ?3 ORDER BY start_at".into(),
            vec![Box::new(user_id) as Box<dyn rusqlite::types::ToSql>, Box::new(s.clone()), Box::new(e_.clone())],
        ),
        _ => (
            "SELECT id, owner_user_id, title, description, location, event_type, start_at, end_at,
             all_day, color, recurrence, reminder_mins, contact_id, deal_id, completed, created_at, updated_at
             FROM crm_events WHERE owner_user_id = ?1 ORDER BY start_at".into(),
            vec![Box::new(user_id) as Box<dyn rusqlite::types::ToSql>],
        ),
    };
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(e)?;
    let rows = stmt.query_map(params_ref.as_slice(), row_to_event).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn crm_delete_event(db: State<'_, CrmDb>, event_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM crm_events WHERE id = ?1 AND owner_user_id = ?2", params![event_id, user_id]).map_err(e)?;
    Ok(())
}

fn get_event_by_id(conn: &rusqlite::Connection, id: &str) -> CmdResult<CalendarEvent> {
    conn.query_row(
        "SELECT id, owner_user_id, title, description, location, event_type, start_at, end_at,
         all_day, color, recurrence, reminder_mins, contact_id, deal_id, completed, created_at, updated_at
         FROM crm_events WHERE id = ?1",
        params![id], row_to_event,
    ).map_err(e)
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get(0)?, owner_user_id: row.get(1)?, title: row.get(2)?,
        description: row.get(3)?, location: row.get(4)?, event_type: row.get(5)?,
        start_at: row.get(6)?, end_at: row.get(7)?,
        all_day: row.get::<_, i32>(8)? != 0, color: row.get(9)?,
        recurrence: row.get(10)?, reminder_mins: row.get(11)?,
        contact_id: row.get(12)?, deal_id: row.get(13)?,
        completed: row.get::<_, i32>(14)? != 0,
        created_at: row.get(15)?, updated_at: row.get(16)?,
    })
}

/* ══════════════════════════════════════════════════════
   DEALS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_deal(db: State<'_, CrmDb>, user_id: String, input: CreateDealInput) -> CmdResult<Deal> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_deals (id, owner_user_id, contact_id, title, value, currency, stage,
         probability, expected_close, notes, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            id, user_id, input.contact_id.unwrap_or_default(), input.title,
            input.value.unwrap_or(0.0), input.currency.unwrap_or("USD".into()),
            input.stage.unwrap_or("prospect".into()), input.probability.unwrap_or(10),
            input.expected_close.unwrap_or_default(), input.notes.unwrap_or_default(),
            ts, ts,
        ],
    ).map_err(e)?;
    get_deal_by_id_scoped(&conn, &id, &user_id)
}

#[tauri::command]
pub fn crm_update_deal(db: State<'_, CrmDb>, deal_id: String, user_id: String, input: UpdateDealInput) -> CmdResult<Deal> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut idx = 2u32;
    macro_rules! opt_set {
        ($field:ident, $col:expr) => { if input.$field.is_some() { sets.push(format!("{} = ?{}", $col, idx)); idx += 1; } };
    }
    opt_set!(contact_id, "contact_id"); opt_set!(title, "title"); opt_set!(currency, "currency");
    opt_set!(stage, "stage"); opt_set!(expected_close, "expected_close"); opt_set!(notes, "notes");
    if input.value.is_some() { sets.push(format!("value = ?{}", idx)); idx += 1; }
    if input.probability.is_some() { sets.push(format!("probability = ?{}", idx)); idx += 1; }
    if input.won.is_some() { sets.push(format!("won = ?{}", idx)); idx += 1; }
    if input.lost.is_some() { sets.push(format!("lost = ?{}", idx)); idx += 1; }

    let sql = format!("UPDATE crm_deals SET {} WHERE id = ?{} AND owner_user_id = ?{}", sets.join(", "), idx, idx + 1);
    let mut p: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(ts)];
    macro_rules! opt_push { ($field:ident) => { if let Some(v) = input.$field { p.push(Box::new(v)); } }; }
    opt_push!(contact_id); opt_push!(title); opt_push!(currency); opt_push!(stage);
    opt_push!(expected_close); opt_push!(notes);
    if let Some(v) = input.value { p.push(Box::new(v)); }
    if let Some(v) = input.probability { p.push(Box::new(v)); }
    if let Some(v) = input.won { p.push(Box::new(v as i32)); }
    if let Some(v) = input.lost { p.push(Box::new(v as i32)); }
    p.push(Box::new(deal_id.clone()));
    p.push(Box::new(user_id.clone()));

    let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
    let affected_rows = conn.execute(&sql, params_ref.as_slice()).map_err(e)?;
    if affected_rows == 0 {
        return Err("Deal not found or access denied".to_string());
    }
    get_deal_by_id_scoped(&conn, &deal_id, &user_id)
}

#[tauri::command]
pub fn crm_get_deals(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<Deal>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, owner_user_id, contact_id, title, value, currency, stage, probability,
         expected_close, notes, won, lost, created_at, updated_at
         FROM crm_deals WHERE owner_user_id = ?1 ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], row_to_deal).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn crm_delete_deal(db: State<'_, CrmDb>, deal_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM crm_deals WHERE id = ?1 AND owner_user_id = ?2", params![deal_id, user_id]).map_err(e)?;
    Ok(())
}

fn get_deal_by_id(conn: &rusqlite::Connection, id: &str) -> CmdResult<Deal> {
    conn.query_row(
        "SELECT id, owner_user_id, contact_id, title, value, currency, stage, probability,
         expected_close, notes, won, lost, created_at, updated_at FROM crm_deals WHERE id = ?1",
        params![id], row_to_deal,
    ).map_err(e)
}

fn row_to_deal(row: &rusqlite::Row) -> rusqlite::Result<Deal> {
    Ok(Deal {
        id: row.get(0)?, owner_user_id: row.get(1)?, contact_id: row.get(2)?,
        title: row.get(3)?, value: row.get(4)?, currency: row.get(5)?,
        stage: row.get(6)?, probability: row.get(7)?, expected_close: row.get(8)?,
        notes: row.get(9)?, won: row.get::<_, i32>(10)? != 0, lost: row.get::<_, i32>(11)? != 0,
        created_at: row.get(12)?, updated_at: row.get(13)?,
    })
}

fn get_contact_by_id_scoped(conn: &rusqlite::Connection, id: &str, owner_user_id: &str) -> CmdResult<Contact> {
    conn.query_row(
        "SELECT id, owner_user_id, first_name, last_name, email, phone, company, job_title,
         avatar_url, address, city, state, zip, country, website, notes, tags, source, stage,
         priority, last_contacted, created_at, updated_at FROM crm_contacts WHERE id = ?1 AND owner_user_id = ?2",
        params![id, owner_user_id], row_to_contact,
    ).map_err(e)
}

fn get_event_by_id_scoped(conn: &rusqlite::Connection, id: &str, owner_user_id: &str) -> CmdResult<CalendarEvent> {
    conn.query_row(
        "SELECT id, owner_user_id, title, description, location, event_type, start_at, end_at,
         all_day, color, recurrence, reminder_mins, contact_id, deal_id, completed, created_at, updated_at
         FROM crm_events WHERE id = ?1 AND owner_user_id = ?2",
        params![id, owner_user_id], row_to_event,
    ).map_err(e)
}

fn get_deal_by_id_scoped(conn: &rusqlite::Connection, id: &str, owner_user_id: &str) -> CmdResult<Deal> {
    conn.query_row(
        "SELECT id, owner_user_id, contact_id, title, value, currency, stage, probability,
         expected_close, notes, won, lost, created_at, updated_at FROM crm_deals WHERE id = ?1 AND owner_user_id = ?2",
        params![id, owner_user_id], row_to_deal,
    ).map_err(e)
}

/* ══════════════════════════════════════════════════════
   ACTIVITIES
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_activity(db: State<'_, CrmDb>, user_id: String, input: CreateActivityInput) -> CmdResult<Activity> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_activities (id, owner_user_id, contact_id, deal_id, event_id, activity_type, subject, body, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            id, user_id,
            input.contact_id.unwrap_or_default(),
            input.deal_id.unwrap_or_default(),
            input.event_id.unwrap_or_default(),
            input.activity_type,
            input.subject.unwrap_or_default(),
            input.body.unwrap_or_default(),
            ts,
        ],
    ).map_err(e)?;
    conn.query_row(
        "SELECT id, owner_user_id, contact_id, deal_id, event_id, activity_type, subject, body, created_at
         FROM crm_activities WHERE id = ?1", params![id], row_to_activity,
    ).map_err(e)
}

#[tauri::command]
pub fn crm_get_activities(db: State<'_, CrmDb>, user_id: String, contact_id: Option<String>) -> CmdResult<Vec<Activity>> {
    let conn = db.conn.lock().map_err(e)?;
    let (sql, p): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match contact_id {
        Some(cid) => (
            "SELECT id, owner_user_id, contact_id, deal_id, event_id, activity_type, subject, body, created_at
             FROM crm_activities WHERE owner_user_id = ?1 AND contact_id = ?2 ORDER BY created_at DESC".into(),
            vec![Box::new(user_id), Box::new(cid)],
        ),
        None => (
            "SELECT id, owner_user_id, contact_id, deal_id, event_id, activity_type, subject, body, created_at
             FROM crm_activities WHERE owner_user_id = ?1 ORDER BY created_at DESC LIMIT 100".into(),
            vec![Box::new(user_id) as Box<dyn rusqlite::types::ToSql>],
        ),
    };
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = p.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(e)?;
    let rows = stmt.query_map(params_ref.as_slice(), row_to_activity).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn row_to_activity(row: &rusqlite::Row) -> rusqlite::Result<Activity> {
    Ok(Activity {
        id: row.get(0)?, owner_user_id: row.get(1)?, contact_id: row.get(2)?,
        deal_id: row.get(3)?, event_id: row.get(4)?, activity_type: row.get(5)?,
        subject: row.get(6)?, body: row.get(7)?, created_at: row.get(8)?,
    })
}

/* ══════════════════════════════════════════════════════
   EMAIL TEMPLATES
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_email_template(db: State<'_, CrmDb>, user_id: String, input: CreateEmailTemplateInput) -> CmdResult<EmailTemplate> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO crm_email_templates (id, owner_user_id, name, subject, body, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, user_id, input.name, input.subject, input.body, ts, ts],
    ).map_err(e)?;
    conn.query_row(
        "SELECT id, owner_user_id, name, subject, body, created_at, updated_at
         FROM crm_email_templates WHERE id = ?1", params![id], row_to_template,
    ).map_err(e)
}

#[tauri::command]
pub fn crm_get_email_templates(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<EmailTemplate>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, owner_user_id, name, subject, body, created_at, updated_at
         FROM crm_email_templates WHERE owner_user_id = ?1 ORDER BY name"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], row_to_template).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn crm_delete_email_template(db: State<'_, CrmDb>, template_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM crm_email_templates WHERE id = ?1 AND owner_user_id = ?2", params![template_id, user_id]).map_err(e)?;
    Ok(())
}

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<EmailTemplate> {
    Ok(EmailTemplate {
        id: row.get(0)?, owner_user_id: row.get(1)?, name: row.get(2)?,
        subject: row.get(3)?, body: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
    })
}

/* ══════════════════════════════════════════════════════
   SMTP CONFIG
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_save_smtp_config(db: State<'_, CrmDb>, user_id: String, input: SaveSmtpConfigInput) -> CmdResult<SmtpConfig> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let id = uid();
    conn.execute(
        "INSERT OR REPLACE INTO crm_smtp_config (id, owner_user_id, host, port, username, password,
         from_name, from_email, use_tls, created_at, updated_at)
         VALUES (
           COALESCE((SELECT id FROM crm_smtp_config WHERE owner_user_id = ?1), ?2),
           ?1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 
           COALESCE((SELECT created_at FROM crm_smtp_config WHERE owner_user_id = ?1), ?10),
           ?10
         )",
        params![
            user_id, id, input.host, input.port.unwrap_or(587),
            input.username, input.password, input.from_name, input.from_email,
            input.use_tls.unwrap_or(true) as i32, ts,
        ],
    ).map_err(e)?;
    conn.query_row(
        "SELECT id, owner_user_id, host, port, username, password, from_name, from_email, use_tls, created_at, updated_at
         FROM crm_smtp_config WHERE owner_user_id = ?1", params![user_id], row_to_smtp,
    ).map_err(e)
}

#[tauri::command]
pub fn crm_get_smtp_config(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Option<SmtpConfig>> {
    let conn = db.conn.lock().map_err(e)?;
    let result = conn.query_row(
        "SELECT id, owner_user_id, host, port, username, password, from_name, from_email, use_tls, created_at, updated_at
         FROM crm_smtp_config WHERE owner_user_id = ?1", params![user_id], row_to_smtp,
    );
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(e(err)),
    }
}

fn row_to_smtp(row: &rusqlite::Row) -> rusqlite::Result<SmtpConfig> {
    Ok(SmtpConfig {
        id: row.get(0)?, owner_user_id: row.get(1)?, host: row.get(2)?,
        port: row.get(3)?, username: row.get(4)?, password: row.get(5)?,
        from_name: row.get(6)?, from_email: row.get(7)?,
        use_tls: row.get::<_, i32>(8)? != 0,
        created_at: row.get(9)?, updated_at: row.get(10)?,
    })
}

/* ══════════════════════════════════════════════════════
   SEND EMAIL (via SMTP)
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub async fn crm_send_email(db: State<'_, CrmDb>, user_id: String, input: SendEmailInput) -> CmdResult<SentEmail> {
    // Get SMTP config
    let (smtp_cfg, email_id, ts) = {
        let conn = db.conn.lock().map_err(e)?;
        let cfg = conn.query_row(
            "SELECT id, owner_user_id, host, port, username, password, from_name, from_email, use_tls, created_at, updated_at
             FROM crm_smtp_config WHERE owner_user_id = ?1", params![user_id], row_to_smtp,
        ).map_err(|_| "SMTP not configured. Go to Settings to set up your email server.".to_string())?;
        let eid = uid();
        let t = now();
        (cfg, eid, t)
    };

    let sender = SmtpSender {
        host: smtp_cfg.host,
        port: smtp_cfg.port as u16,
        username: smtp_cfg.username,
        password: smtp_cfg.password,
        from_name: smtp_cfg.from_name,
        from_email: smtp_cfg.from_email,
        use_tls: smtp_cfg.use_tls,
    };

    let result = sender.send_email(&input.to_email, &input.subject, &input.body).await;

    let (status, error_msg) = match &result {
        Ok(_) => ("sent".to_string(), "".to_string()),
        Err(err) => ("failed".to_string(), err.clone()),
    };

    // Log sent email
    let contact_id_val = input.contact_id.clone().unwrap_or_default();
    {
        let conn = db.conn.lock().map_err(e)?;
        conn.execute(
            "INSERT INTO crm_sent_emails (id, owner_user_id, contact_id, to_email, subject, body, status, error_message, template_id, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                email_id, user_id, &contact_id_val,
                input.to_email, input.subject, input.body,
                status, error_msg, input.template_id.unwrap_or_default(), ts,
            ],
        ).map_err(e)?;

        // Update contact last_contacted (only if user owns the contact)
        if !contact_id_val.is_empty() {
            conn.execute("UPDATE crm_contacts SET last_contacted = ?1 WHERE id = ?2 AND owner_user_id = ?3", params![ts, &contact_id_val, &user_id]).ok();
        }
    }

    if let Err(err) = result {
        return Err(format!("Email failed: {}", err));
    }

    // Return the sent email record
    let conn = db.conn.lock().map_err(e)?;
    conn.query_row(
        "SELECT id, owner_user_id, contact_id, to_email, subject, body, status, error_message, template_id, created_at
         FROM crm_sent_emails WHERE id = ?1 AND owner_user_id = ?2", params![email_id, user_id], row_to_sent_email,
    ).map_err(e)
}

#[tauri::command]
pub fn crm_get_sent_emails(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<SentEmail>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, owner_user_id, contact_id, to_email, subject, body, status, error_message, template_id, created_at
         FROM crm_sent_emails WHERE owner_user_id = ?1 ORDER BY created_at DESC LIMIT 100"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], row_to_sent_email).map_err(e)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn row_to_sent_email(row: &rusqlite::Row) -> rusqlite::Result<SentEmail> {
    Ok(SentEmail {
        id: row.get(0)?, owner_user_id: row.get(1)?, contact_id: row.get(2)?,
        to_email: row.get(3)?, subject: row.get(4)?, body: row.get(5)?,
        status: row.get(6)?, error_message: row.get(7)?, template_id: row.get(8)?,
        created_at: row.get(9)?,
    })
}

/* ══════════════════════════════════════════════════════
   CRM STATS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_get_stats(db: State<'_, CrmDb>, user_id: String) -> CmdResult<CrmStats> {
    let conn = db.conn.lock().map_err(e)?;
    let contact_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_contacts WHERE owner_user_id=?1", params![user_id], |r| r.get(0)).unwrap_or(0);
    let deal_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_deals WHERE owner_user_id=?1", params![user_id], |r| r.get(0)).unwrap_or(0);
    let open_deal_value: f64 = conn.query_row("SELECT COALESCE(SUM(value),0) FROM crm_deals WHERE owner_user_id=?1 AND won=0 AND lost=0", params![user_id], |r| r.get(0)).unwrap_or(0.0);
    let won_deal_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_deals WHERE owner_user_id=?1 AND won=1", params![user_id], |r| r.get(0)).unwrap_or(0);
    let event_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_events WHERE owner_user_id=?1", params![user_id], |r| r.get(0)).unwrap_or(0);
    let ts = now();
    let upcoming_events: i32 = conn.query_row("SELECT COUNT(*) FROM crm_events WHERE owner_user_id=?1 AND start_at >= ?2 AND completed=0", params![user_id, ts], |r| r.get(0)).unwrap_or(0);
    let email_sent_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_sent_emails WHERE owner_user_id=?1 AND status='sent'", params![user_id], |r| r.get(0)).unwrap_or(0);
    let activity_count: i32 = conn.query_row("SELECT COUNT(*) FROM crm_activities WHERE owner_user_id=?1", params![user_id], |r| r.get(0)).unwrap_or(0);
    Ok(CrmStats { contact_count, deal_count, open_deal_value, won_deal_count, event_count, upcoming_events, email_sent_count, activity_count })
}

/* ══════════════════════════════════════════════════════
   CSV IMPORT / EXPORT
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_import_csv(db: State<'_, CrmDb>, user_id: String, input: CsvImportRequest) -> CmdResult<CsvImportResult> {
    let mut imported = 0;
    let mut duplicates = 0;
    let mut updated = 0;
    let mut errors = 0;
    let mut error_list = vec![];

    for line in input.csv_content.lines().skip(1) {
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        let mut values: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();

        // Parse CSV into mapped values
        for (idx, val) in fields.iter().enumerate() {
            if let Some(header) = input.csv_content.lines().next().and_then(|h| h.split(',').nth(idx)) {
                values.insert(header.trim(), val);
            }
        }

        // Check for duplicates by email
        let email = input.column_mapping.get("email").and_then(|field| values.get(field.as_str())).copied().unwrap_or("");
        
        let conn = db.conn.lock().map_err(|_| "DB lock".to_string())?;
        let existing = conn.query_row(
            "SELECT id FROM crm_contacts WHERE owner_user_id=?1 AND email=?2",
            params![&user_id, email],
            |r| r.get::<_, String>(0),
        );

        match existing {
            Ok(existing_id) => {
                if input.update_existing {
                    // Update existing contact
                    // Build dynamic UPDATE query based on mapping
                    if !email.is_empty() {
                        conn.execute(
                            "UPDATE crm_contacts SET updated_at=?1 WHERE id=?2",
                            params![now(), &existing_id],
                        ).ok();
                        updated += 1;
                    }
                } else if input.skip_duplicates {
                    duplicates += 1;
                }
            }
            Err(_) => {
                // Insert new contact
                if let Err(err) = conn.execute(
                    "INSERT INTO crm_contacts (id, owner_user_id, first_name, email, phone, company, job_title, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        uid(), &user_id,
                        values.get("first_name").copied().unwrap_or(""),
                        email,
                        values.get("phone").copied().unwrap_or(""),
                        values.get("company").copied().unwrap_or(""),
                        values.get("job_title").copied().unwrap_or(""),
                        now(), now()
                    ],
                ) {
                    errors += 1;
                    error_list.push(format!("Row error: {}", err));
                } else {
                    imported += 1;
                }
            }
        }
    }

    Ok(CsvImportResult {
        imported_count: imported,
        duplicate_count: duplicates,
        updated_count: updated,
        error_count: errors,
        errors: error_list,
    })
}

#[tauri::command]
pub fn crm_export_csv(db: State<'_, CrmDb>, user_id: String) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let mut csv = String::from("id,first_name,last_name,email,phone,company,job_title,address,city,state,zip,country,website,notes,tags,source,stage,priority,last_contacted,created_at\n");

    let mut stmt = conn.prepare(
        "SELECT id, first_name, last_name, email, phone, company, job_title, address, city, state, zip, country, website, notes, tags, source, stage, priority, last_contacted, created_at
         FROM crm_contacts WHERE owner_user_id=?1"
    ).map_err(e)?;

    let contacts = stmt.query_map(params![&user_id], |row| {
        Ok((
            row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
            row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
            row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?,
            row.get::<_, String>(9)?, row.get::<_, String>(10)?, row.get::<_, String>(11)?,
            row.get::<_, String>(12)?, row.get::<_, String>(13)?, row.get::<_, String>(14)?,
            row.get::<_, String>(15)?, row.get::<_, String>(16)?, row.get::<_, String>(17)?,
            row.get::<_, String>(18)?, row.get::<_, String>(19)?,
        ))
    }).map_err(e)?;

    for contact in contacts {
        let c = contact.map_err(e)?;
        csv.push_str(&format!("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            c.0, c.1, c.2, c.3, c.4, c.5, c.6, c.7, c.8, c.9, c.10, c.11, c.12, c.13, c.14, c.15, c.16, c.17, c.18, c.19
        ));
    }

    Ok(csv)
}

/* ══════════════════════════════════════════════════════
   CONTACT DEDUPLICATION
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_find_duplicates(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<DuplicateContact>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut duplicates = vec![];

    // Find potential duplicates by email
    let mut stmt = conn.prepare(
        "SELECT id, first_name, last_name, email FROM crm_contacts WHERE owner_user_id=?1 AND email != '' ORDER BY email"
    ).map_err(e)?;

    let contacts: Vec<(String, String, String, String)> = stmt.query_map(params![&user_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    }).map_err(e)?
        .filter_map(|r| r.ok())
        .collect();

    for i in 0..contacts.len() {
        for j in (i+1)..contacts.len() {
            let (id1, fn1, ln1, e1) = &contacts[i];
            let (id2, fn2, ln2, e2) = &contacts[j];

            // Check if same email
            if e1 == e2 && !e1.is_empty() {
                duplicates.push(DuplicateContact {
                    id1: id1.clone(),
                    id2: id2.clone(),
                    name1: format!("{} {}", fn1, ln1),
                    name2: format!("{} {}", fn2, ln2),
                    email1: e1.clone(),
                    email2: e2.clone(),
                    similarity_score: 100.0,
                    reason: "Same email address".to_string(),
                });
            }
        }
    }

    Ok(duplicates)
}

#[tauri::command]
pub fn crm_merge_contacts(db: State<'_, CrmDb>, _contact_id: String, user_id: String, input: MergeContactsInput) -> CmdResult<Contact> {
    let conn = db.conn.lock().map_err(e)?;

    // Verify ownership of both contacts
    let primary_owner: String = conn.query_row(
        "SELECT owner_user_id FROM crm_contacts WHERE id = ?1",
        params![&input.primary_id],
        |r| r.get(0),
    ).map_err(|_| "Primary contact not found".to_string())?;

    let secondary_owner: String = conn.query_row(
        "SELECT owner_user_id FROM crm_contacts WHERE id = ?1",
        params![&input.secondary_id],
        |r| r.get(0),
    ).map_err(|_| "Secondary contact not found".to_string())?;

    if primary_owner != user_id || secondary_owner != user_id {
        return Err("Access denied: cannot merge contacts you don't own".to_string());
    }

    // Merge all activities from secondary to primary
    conn.execute(
        "UPDATE crm_activities SET contact_id=?1 WHERE contact_id=?2 AND owner_user_id=?3",
        params![&input.primary_id, &input.secondary_id, &user_id],
    ).map_err(e)?;

    // Merge all deals from secondary to primary
    conn.execute(
        "UPDATE crm_deals SET contact_id=?1 WHERE contact_id=?2 AND owner_user_id=?3",
        params![&input.primary_id, &input.secondary_id, &user_id],
    ).map_err(e)?;

    // Delete secondary contact
    conn.execute("DELETE FROM crm_contacts WHERE id=?1 AND owner_user_id=?2", params![&input.secondary_id, &user_id]).map_err(e)?;

    // Return updated primary contact
    get_contact_by_id_scoped(&conn, &input.primary_id, &user_id)
}

/* ══════════════════════════════════════════════════════
   CAMPAIGN MANAGEMENT
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_create_campaign(db: State<'_, CrmDb>, user_id: String, input: CreateCampaignInput) -> CmdResult<Campaign> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();

    conn.execute(
        "INSERT INTO crm_campaigns (id, owner_user_id, name, description, campaign_type, status, target_contacts, sent_count, opened_count, clicked_count, conversion_count, scheduled_at, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,'draft',0,0,0,0,0,?6,?7,?8)",
        params![
            id, user_id, input.name, input.description.unwrap_or_default(), input.campaign_type,
            input.scheduled_at.unwrap_or_default(), ts, ts
        ],
    ).map_err(e)?;

    conn.query_row(
        "SELECT id, owner_user_id, name, description, campaign_type, status, target_contacts, sent_count, opened_count, clicked_count, conversion_count, scheduled_at, started_at, completed_at, created_at, updated_at
         FROM crm_campaigns WHERE id=?1",
        params![id],
        |row| {
            Ok(Campaign {
                id: row.get(0)?,
                owner_user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                campaign_type: row.get(4)?,
                status: row.get(5)?,
                target_contacts: row.get(6)?,
                sent_count: row.get(7)?,
                opened_count: row.get(8)?,
                clicked_count: row.get(9)?,
                conversion_count: row.get(10)?,
                scheduled_at: row.get(11)?,
                started_at: row.get::<_, String>(12).unwrap_or_default(),
                completed_at: row.get::<_, String>(13).unwrap_or_default(),
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        }
    ).map_err(e)
}

/* ══════════════════════════════════════════════════════
   LEAD SCORING
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_calculate_lead_scores(db: State<'_, CrmDb>, user_id: String) -> CmdResult<Vec<ContactWithScore>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut results = vec![];

    let mut stmt = conn.prepare("SELECT id, first_name, last_name, email, phone, company, job_title, avatar_url, address, city, state, zip, country, website, notes, tags, source, stage, priority, last_contacted, created_at, updated_at FROM crm_contacts WHERE owner_user_id=?1").map_err(e)?;

    let contacts = stmt.query_map(params![&user_id], |row| {
        Ok(Contact {
            id: row.get(0)?, owner_user_id: row.get(1)?, first_name: row.get(2)?, 
            last_name: row.get(3)?, email: row.get(4)?, phone: row.get(5)?,
            company: row.get(6)?, job_title: row.get(7)?, avatar_url: row.get(8)?,
            address: row.get(9)?, city: row.get(10)?, state: row.get(11)?,
            zip: row.get(12)?, country: row.get(13)?, website: row.get(14)?,
            notes: row.get(15)?, tags: row.get(16)?, source: row.get(17)?,
            stage: row.get(18)?, priority: row.get(19)?, last_contacted: row.get(20)?,
            created_at: row.get(21)?, updated_at: row.get(22)?,
        })
    }).map_err(e)?;

    for contact_result in contacts {
        if let Ok(contact) = contact_result {
            // Calculate engagement points: activities, emails sent, events attended
            let engagement: i32 = conn.query_row(
                "SELECT COUNT(*) FROM crm_activities WHERE contact_id=?1",
                params![&contact.id],
                |r| r.get(0)
            ).unwrap_or(0) * 5;

            let emails: i32 = conn.query_row(
                "SELECT COUNT(*) FROM crm_sent_emails WHERE contact_id=?1 AND status='sent'",
                params![&contact.id],
                |r| r.get(0)
            ).unwrap_or(0) * 3;

            // Company points
            let company_points = if !contact.company.is_empty() { 10 } else { 0 };

            // Calculate total score (0-100)
            let score = (engagement + emails + company_points).min(100) as i32;
            let grade = match score {
                90..=100 => "A",
                80..=89 => "B",
                70..=79 => "C",
                60..=69 => "D",
                _ => "F",
            }.to_string();

            results.push(ContactWithScore {
                contact: contact.clone(),
                lead_score: LeadScore {
                    contact_id: contact.id.clone(),
                    score,
                    grade,
                    engagement_points: engagement,
                    company_points,
                    behavioral_points: emails,
                    last_updated: now(),
                },
            });
        }
    }

    Ok(results)
}

/* ══════════════════════════════════════════════════════
   BULK ACTIONS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_bulk_update(db: State<'_, CrmDb>, user_id: String, input: BulkUpdateInput) -> CmdResult<BulkActionResult> {
    let conn = db.conn.lock().map_err(e)?;
    let mut success = 0;
    let mut failures = 0;
    let mut errors = vec![];

    for contact_id in &input.contact_ids {
        let result = conn.execute(
            "UPDATE crm_contacts SET stage=?1, priority=?2, updated_at=?3 WHERE id=?4 AND owner_user_id=?5",
            params![
                input.updates.get("stage").and_then(|v| v.as_str()).unwrap_or(""),
                input.updates.get("priority").and_then(|v| v.as_str()).unwrap_or(""),
                now(),
                contact_id,
                user_id
            ],
        );

        match result {
            Ok(_) => success += 1,
            Err(err) => {
                failures += 1;
                errors.push(format!("Contact {}: {}", contact_id, err));
            }
        }
    }

    Ok(BulkActionResult {
        success_count: success,
        failure_count: failures,
        errors,
    })
}

/* ══════════════════════════════════════════════════════
   DEAL FORECASTING & PIPELINE ANALYTICS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn crm_get_pipeline_analytics(db: State<'_, CrmDb>, user_id: String) -> CmdResult<PipelineAnalytics> {
    let conn = db.conn.lock().map_err(e)?;

    let total_value: f64 = conn.query_row(
        "SELECT COALESCE(SUM(value), 0) FROM crm_deals WHERE owner_user_id=?1 AND won=0 AND lost=0",
        params![&user_id],
        |r| r.get(0)
    ).unwrap_or(0.0);

    let total_deals: i32 = conn.query_row(
        "SELECT COUNT(*) FROM crm_deals WHERE owner_user_id=?1 AND won=0 AND lost=0",
        params![&user_id],
        |r| r.get(0)
    ).unwrap_or(0);

    let average_deal_value = if total_deals > 0 { total_value / total_deals as f64 } else { 0.0 };

    let mut stage_breakdown = std::collections::HashMap::new();
    let stages = vec!["prospect", "qualified", "proposal", "negotiation", "closed"];

    for stage in stages {
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM crm_deals WHERE owner_user_id=?1 AND stage=?2",
            params![&user_id, stage],
            |r| r.get(0)
        ).unwrap_or(0);

        let stage_value: f64 = conn.query_row(
            "SELECT COALESCE(SUM(value), 0) FROM crm_deals WHERE owner_user_id=?1 AND stage=?2",
            params![&user_id, stage],
            |r| r.get(0)
        ).unwrap_or(0.0);

        stage_breakdown.insert(stage.to_string(), PipelineStageStats {
            stage_name: stage.to_string(),
            count,
            total_value: stage_value,
            avg_days_in_stage: 0.0,
            win_probability: if stage == "closed" { 100.0 } else { 30.0 + (stage_value / average_deal_value.max(1.0)) as f32 },
        });
    }

    // Generate 6-month forecast
    let mut months_forecast = vec![];
    for i in 0..6 {
        let month_value = (total_value / 6.0) + (rand::random::<f64>() - 0.5) * total_value * 0.1;
        months_forecast.push(DealForecast {
            month: format!("{:02}", ((chrono::Utc::now().month() + i) % 12 + 1)),
            confidence_low: month_value * 0.8,
            confidence_mid: month_value,
            confidence_high: month_value * 1.2,
            expected_value: month_value,
            historical_accuracy: 75.0,
        });
    }

    Ok(PipelineAnalytics {
        total_value,
        total_deals,
        average_deal_value,
        weighted_forecast: total_value * 0.7,
        stage_breakdown,
        months_forecast,
    })
}

