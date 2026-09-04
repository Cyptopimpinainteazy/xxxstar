Manual test: Wallet History CSV Export

Purpose
- Verify the `History` view export respects date range and "Include pending" option, and that exported CSV contents match UI selection.

Quick start (dev)
1. Open a terminal in the workspace and start the desktop frontend dev server (from `apps/x3-desktop`).

```bash
cd apps/x3-desktop
npm install    # or yarn / pnpm as used in this project
npm run dev    # start dev server (or the Tauri frontend dev flow)
```

Manual verification steps
1. Launch the app and open the `Wallet` panel → `History`.
2. Observe the top-right controls: `From` (date), `To` (date), and `Include pending` checkbox.
3. Set a wide date range (or clear both date inputs) and toggle `Include pending` on/off. Note the small status text showing `N transactions selected`.
4. Click `Export CSV`.
   - If zero transactions are selected, the button will be disabled.
   - If enabled, a CSV file will be downloaded named like `transactions-YYYY-MM-DD_YYYY-MM-DD-with-pending.csv`.
5. Open the CSV in a text editor or spreadsheet and confirm:
   - The header row includes `id,type,time,amount,symbol,status` and an extra `details` column when `Include pending` is enabled.
   - The rows correspond to the UI list (time, type, status) and the number of rows equals `N transactions selected`.

Edge cases to check
- Transactions with malformed `time` fields should still be included when date filters are empty.
- Very large `details` JSON (when `Include pending`) should be quoted in CSV and not break rows.

Notes
- This is a frontend-only export; it reads from the in-memory `transactions` provided by `useWalletStore()`.
- If you're running a packaged Tauri app, the same flow should work — exported file will land where the OS saves downloads for the webview context.

Optional automation
- Add a Jest test rendering `HistoryView` with mocked `useWalletStore()` and assert that clicking `Export CSV` produces a Blob with expected content. This repo may already use a testing harness; adapt accordingly.
