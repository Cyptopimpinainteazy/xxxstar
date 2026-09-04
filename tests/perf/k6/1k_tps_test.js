import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';

export let errorRate = new Rate('errors');

export let options = {
  vus: 1000,
  duration: '10m',
  thresholds: {
    'errors': ['rate<0.01'], // <1% errors
    'http_req_duration{p(99)<500}': ['p(99)<500'] // placeholder: p99 < 500ms
  }
};

export default function () {
  const url = __ENV.TARGET_URL || 'http://127.0.0.1:8080';
  const payload = JSON.stringify({ method: 'submit_tx', params: { /* sample tx */ } });
  const headers = { 'Content-Type': 'application/json' };

  const res = http.post(url + '/api/v1/tx', payload, { headers });
  const ok = check(res, {
    'status is 200': (r) => r.status === 200
  });
  errorRate.add(!ok);
}
