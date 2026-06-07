# Release Gates

Release candidates must pass:

- Guard checks (`make guard`)
- Test checks (`make test`)
- Audit checks (`make audit`)
- Mainnet readiness gate (`make mainnet-check`)
- Fresh machine check (`make fresh-machine-check`)

Mainnet-ready claims are forbidden unless all gates pass.