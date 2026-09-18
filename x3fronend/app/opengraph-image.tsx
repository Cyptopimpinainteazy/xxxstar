import { ImageResponse } from "next/og";

export const dynamic = "force-static";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          padding: "80px",
          background: "#07080c",
          backgroundImage:
            "radial-gradient(circle at 12% -10%, rgba(79,209,197,0.16), transparent 45%), radial-gradient(circle at 88% 0%, rgba(126,232,222,0.10), transparent 40%)",
          fontFamily: "sans-serif",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 36 }}>
          <svg width="54" height="54" viewBox="0 0 40 40" fill="none">
            <circle cx="20" cy="20" r="3.4" fill="#7ee8de" />
            <g stroke="#4fd1c5" strokeWidth="1.6">
              <ellipse cx="20" cy="20" rx="15.5" ry="6.4" />
              <ellipse cx="20" cy="20" rx="15.5" ry="6.4" transform="rotate(60 20 20)" />
              <ellipse cx="20" cy="20" rx="15.5" ry="6.4" transform="rotate(120 20 20)" />
            </g>
          </svg>
          <div style={{ display: "flex", fontSize: 30, fontWeight: 700, color: "#e9edf5", letterSpacing: -0.5 }}>
            X3 ATOMIC STAR
          </div>
        </div>
        <div style={{ display: "flex", fontSize: 54, fontWeight: 700, color: "#f5f7fb", lineHeight: 1.15, maxWidth: 980 }}>
          Atomic settlement across three execution environments, or none of it happens at all.
        </div>
        <div
          style={{
            display: "flex",
            alignSelf: "flex-start",
            marginTop: 44,
            fontSize: 22,
            fontFamily: "monospace",
            color: "#9aa3b8",
            border: "1px solid rgba(255,255,255,0.16)",
            borderRadius: 10,
            padding: "12px 20px",
          }}
        >
          v0.4 Internal Testnet Candidate — status generated from the repo, not written as marketing copy
        </div>
      </div>
    ),
    { ...size }
  );
}
