const fetch = globalThis.fetch || require('node-fetch');
const nodemailer = require('nodemailer');

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

  const html = `<h2>New lead: ${payload.company || payload.name}</h2><p><strong>Name:</strong> ${payload.name}</p><p><strong>Email:</strong> ${payload.email}</p><p><strong>Company:</strong> ${payload.company}</p><p><strong>Role:</strong> ${payload.role}</p><p><strong>Message:</strong><br/>${payload.message || ''}</p><p><strong>RPC:</strong> ${payload.rpc || '-'}<br/><strong>Requested Demo:</strong> ${payload.request_demo ? 'yes' : 'no'}</p>`;

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
  const html = `<p>Hi ${payload.name || payload.company || 'there'},</p><p>Thanks for joining the X3 presale. We've received your request and will contact you shortly to schedule a demo. If you requested a demo, we'll run a quick benchmark and email you the results.</p><p>Best,<br/>X3 Chain Sales</p>`;
  try {
    await transporter.sendMail({ from: process.env.SALES_FROM || smtpUser, to: payload.email, subject: `Thanks for joining X3 Presale`, html });
    return true;
  } catch (e) {
    console.error('confirmation email failed', e);
    return false;
  }
}

module.exports = { notifyLead, sendConfirmationEmail };
