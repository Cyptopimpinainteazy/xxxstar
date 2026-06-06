const fetch = globalThis.fetch || require('node-fetch');
const nodemailer = require('nodemailer');

// HTML-entity escape for the small set of characters that matter in
// HTML/script contexts. Prevents stored XSS via the email body when the
// payload contains user-supplied strings (name, message, etc.).
// (CodeQL js/client-side-xss #2077)
function escapeHtml(value) {
  if (value === null || value === undefined) return '';
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

async function notifyWebhook(url, payload) {
  try {
    await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
    return true;
  } catch (e) {
    console.error('webhook notify failed', e);
    return false;
  }
}

async function notifyEmail(payload) {
  const smtpHost = process.env.SALES_SMTP_HOST;
  const smtpPort = process.env.SALES_SMTP_PORT || 587;
  const smtpUser = process.env.SALES_SMTP_USER;
  const smtpPass = process.env.SALES_SMTP_PASS;
  const salesTo = process.env.SALES_EMAIL;
  if (!smtpHost || !smtpUser || !smtpPass || !salesTo) {
    console.warn('SMTP not configured; cannot send email');
    return false;
  }

  const transporter = nodemailer.createTransport({ host: smtpHost, port: Number(smtpPort), secure: Number(smtpPort) === 465, auth: { user: smtpUser, pass: smtpPass } });

  // SECURITY: every user-supplied field is HTML-escaped before being embedded
  // in the email body. Without this, an attacker can inject <script> tags or
  // event handlers that fire when a sales rep opens the message in a webmail
  // client. (CodeQL js/client-side-xss #2077)
  const html = `<h2>New lead: ${escapeHtml(payload.company || payload.name)}</h2>`
    + `<p><strong>Name:</strong> ${escapeHtml(payload.name)}</p>`
    + `<p><strong>Email:</strong> ${escapeHtml(payload.email)}</p>`
    + `<p><strong>Company:</strong> ${escapeHtml(payload.company)}</p>`
    + `<p><strong>Role:</strong> ${escapeHtml(payload.role)}</p>`
    + `<p><strong>Message:</strong><br/>${escapeHtml(payload.message || '')}</p>`
    + `<p><strong>RPC:</strong> ${escapeHtml(payload.rpc || '-')}`
    + `<br/><strong>Requested Demo:</strong> ${payload.request_demo ? 'yes' : 'no'}</p>`;

  try {
    await transporter.sendMail({ from: process.env.SALES_FROM || smtpUser, to: salesTo, subject: `New lead: ${payload.company || payload.name}`, html });
    return true;
  } catch (e) {
    console.error('email send failed', e);
    return false;
  }
}

async function notifyLead(payload) {
  // Priority: webhook -> email -> log
  if (process.env.SALES_WEBHOOK) {
    const ok = await notifyWebhook(process.env.SALES_WEBHOOK, payload);
    if (ok) return { via: 'webhook' };
  }
  const ok2 = await notifyEmail(payload);
  if (ok2) return { via: 'email' };

  console.log('lead received', payload);
  return { via: 'log' };
}

async function sendConfirmationEmail(payload) {
  const smtpHost = process.env.SALES_SMTP_HOST;
  const smtpPort = process.env.SALES_SMTP_PORT || 587;
  const smtpUser = process.env.SALES_SMTP_USER;
  const smtpPass = process.env.SALES_SMTP_PASS;
  if (!smtpHost || !smtpUser || !smtpPass || !payload.email) return false;

  const transporter = nodemailer.createTransport({ host: smtpHost, port: Number(smtpPort), secure: Number(smtpPort) === 465, auth: { user: smtpUser, pass: smtpPass } });
  const html = `<p>Hi ${escapeHtml(payload.name || payload.company || 'there')},</p><p>Thanks for joining the X3 presale. We've received your request and will contact you shortly to schedule a demo. If you requested a demo, we'll run a quick benchmark and email you the results.</p><p>Best,<br/>X3 Chain Sales</p>`;
  try {
    await transporter.sendMail({ from: process.env.SALES_FROM || smtpUser, to: payload.email, subject: `Thanks for joining X3 Presale`, html });
    return true;
  } catch (e) {
    console.error('confirmation email failed', e);
    return false;
  }
}

module.exports = { notifyLead, sendConfirmationEmail };
