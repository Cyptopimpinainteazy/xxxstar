"""Conservative HTTPS JSON-feed discovery; operator-approved sources only.

Feed schema: [{id,name,official_url,category,deadline?,amount_usd?,entity_required?,notes?}]
A fetched lead is unverified until human review of the primary official URL.
"""
import argparse
import ipaddress
import json
import pathlib
import socket
import ssl
import urllib.error
import urllib.request
from urllib.parse import urlsplit

import grantops

MAX_BYTES = 2_000_000
MAX_LEADS = 500
CATEGORIES = {'gpu', 'cloud', 'blockchain', 'security', 'research', 'other'}


def _public_host(host):
    if not host or host.lower() == 'localhost' or host.lower().endswith(('.local', '.internal')):
        return False
    try:
        addresses = socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
        return bool(addresses) and all(ipaddress.ip_address(entry[4][0]).is_global for entry in addresses)
    except (socket.gaierror, ValueError):
        return False


def validate_source(url, allowed_hosts):
    parsed = urlsplit(url)
    if (parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password
            or parsed.port not in (None, 443) or parsed.fragment):
        raise ValueError('source must be a plain HTTPS URL on port 443')
    if parsed.hostname.lower() not in allowed_hosts:
        raise ValueError('source hostname not on operator allowlist')
    if not _public_host(parsed.hostname):
        raise ValueError('source hostname does not resolve to public addresses')
    return url


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('redirects are disabled')


def fetch_json(url, allowed_hosts, opener=None):
    validate_source(url, allowed_hosts)
    opener = opener or urllib.request.build_opener(NoRedirect(), urllib.request.HTTPSHandler(context=ssl.create_default_context()))
    request = urllib.request.Request(url, headers={'Accept': 'application/json', 'User-Agent': 'X3GrantHunter/3.0'})
    with opener.open(request, timeout=10) as response:
        if response.status != 200:
            raise ValueError('unexpected HTTP status')
        if 'json' not in response.headers.get('Content-Type', '').lower():
            raise ValueError('expected JSON content type')
        raw = response.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError('feed too large')
    return json.loads(raw)


def normalize(data, source_url):
    if not isinstance(data, list) or len(data) > MAX_LEADS:
        raise ValueError('expected JSON list of up to 500 leads')
    leads = []
    for item in data:
        if not isinstance(item, dict) or item.get('category') not in CATEGORIES:
            raise ValueError('invalid lead category')
        required = ('id', 'name', 'official_url', 'category')
        if any(not isinstance(item.get(k), str) or not item[k].strip() for k in required):
            raise ValueError('invalid lead fields')
        official = urlsplit(item['official_url'])
        if official.scheme != 'https' or not official.hostname or official.username or official.password:
            raise ValueError('invalid official URL')
        clean = {k: item[k] for k in required}
        for key in ('deadline', 'amount_usd', 'entity_required'):
            if key in item:
                clean[key] = item[key]
        clean['notes'] = ('Discovered from ' + source_url + '; verify official listing and eligibility. ' + str(item.get('notes', '')))[:1000]
        leads.append(clean)
    return leads


def run(config, db_path=None, *, dry_run=True, fetcher=fetch_json):
    if not isinstance(config, dict) or not isinstance(config.get('sources'), list):
        raise ValueError('config must contain sources list')
    all_leads, report = [], []
    for source in config['sources']:
        url = source['url']
        hosts = {str(h).lower() for h in source['allowed_hosts']}
        try:
            leads = normalize(fetcher(url, hosts), url)
            all_leads.extend(leads)
            report.append({'source': url, 'count': len(leads), 'status': 'fetched_unverified'})
        except (ValueError, KeyError, TypeError, OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
            report.append({'source': url, 'count': 0, 'status': 'error', 'reason': str(exc)})
    unique = list({lead['id']: lead for lead in all_leads}.values())
    if not dry_run and unique:
        grantops.import_leads(unique, db_path)
    return {'dry_run': dry_run, 'leads': unique, 'sources': report, 'count': len(unique), 'eligibility_verified': False}


def main():
    parser = argparse.ArgumentParser(description='Review-only X3 grant feed discovery')
    parser.add_argument('config', type=pathlib.Path)
    parser.add_argument('--db', type=pathlib.Path)
    parser.add_argument('--import-reviewed', action='store_true', help='import unverified leads into review pipeline; does not submit applications')
    args = parser.parse_args()
    print(json.dumps(run(json.loads(args.config.read_text()), args.db, dry_run=not args.import_reviewed), indent=2))


if __name__ == '__main__':
    main()
