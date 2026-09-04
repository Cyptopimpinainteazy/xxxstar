## GitHub Copilot Chat

- Extension: 0.37.6 (prod)
- VS Code: 1.109.4 (c3a26841a84f20dfe0850d0a5a9bd01da4f003ea)
- OS: linux 5.15.0-170-generic x64
- GitHub Account: Cyptopimpinainteazy

## Network

User Settings:
```json
  "http.systemCertificatesNode": true,
  "github.copilot.advanced.debug.useElectronFetcher": true,
  "github.copilot.advanced.debug.useNodeFetcher": false,
  "github.copilot.advanced.debug.useNodeFetchFetcher": true
```

Connecting to https://api.github.com:
- DNS ipv4 Lookup: 140.82.113.5 (13 ms)
- DNS ipv6 Lookup: Error (62 ms): getaddrinfo ENOTFOUND api.github.com
- Proxy URL: None (1 ms)
- Electron fetch (configured): HTTP 200 (66 ms)
- Node.js https: HTTP 200 (247 ms)
- Node.js fetch: HTTP 200 (271 ms)

Connecting to https://api.individual.githubcopilot.com/_ping:
- DNS ipv4 Lookup: 140.82.114.21 (15 ms)
- DNS ipv6 Lookup: Error (15 ms): getaddrinfo ENOTFOUND api.individual.githubcopilot.com
- Proxy URL: None (4 ms)
- Electron fetch (configured): HTTP 200 (60 ms)
- Node.js https: HTTP 200 (248 ms)
- Node.js fetch: HTTP 200 (255 ms)

Connecting to https://proxy.individual.githubcopilot.com/_ping:
- DNS ipv4 Lookup: 138.91.182.224 (14 ms)
- DNS ipv6 Lookup: Error (15 ms): getaddrinfo ENOTFOUND proxy.individual.githubcopilot.com
- Proxy URL: None (24 ms)
- Electron fetch (configured): HTTP 200 (161 ms)
- Node.js https: HTTP 200 (205 ms)
- Node.js fetch: HTTP 200 (192 ms)

Connecting to https://mobile.events.data.microsoft.com: HTTP 404 (32 ms)
Connecting to https://dc.services.visualstudio.com: HTTP 404 (219 ms)
Connecting to https://copilot-telemetry.githubusercontent.com/_ping: HTTP 200 (234 ms)
Connecting to https://telemetry.individual.githubcopilot.com/_ping: HTTP 200 (240 ms)
Connecting to https://default.exp-tas.com: HTTP 400 (143 ms)

Number of system certificates: 435

## Documentation

In corporate networks: [Troubleshooting firewall settings for GitHub Copilot](https://docs.github.com/en/copilot/troubleshooting-github-copilot/troubleshooting-firewall-settings-for-github-copilot).