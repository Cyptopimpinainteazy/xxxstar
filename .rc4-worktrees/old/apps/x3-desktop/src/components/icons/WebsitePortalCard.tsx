/**
 * WebsitePortalCard — prominent gateway to x3star.net website
 *
 * Appears in the center of the desktop, inviting users to visit the
 * full informational website. Complements the app-focused Tauri interface.
 */
import React, { useState, useCallback } from "react";

const WebsitePortalCard: React.FC = () => {
  const [hovered, setHovered] = useState(false);

  const handleClick = useCallback(async () => {
    // Open website in default browser
    window.open("https://x3star.net", "_blank");
  }, []);

  const accentColor = "#00d4aa";

  return (
    <div
      className="cursor-pointer select-none"
      style={{ perspective: 1000, width: 320, height: 200 }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={handleClick}
      role="button"
      tabIndex={0}
      aria-label="Visit X3STAR website — click to open in browser"
      onKeyDown={(e) => {
        if (e.key === "Enter") handleClick();
      }}
    >
      <div
        style={{
          width: "100%",
          height: "100%",
          position: "relative",
          transformStyle: "preserve-3d",
          transition: "all 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
          transform: hovered ? "scale(1.05) translateY(-8px)" : "scale(1)",
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: 16,
            border: `2px solid ${accentColor}66`,
            background: `linear-gradient(135deg, rgba(0,30,35,0.8) 0%, rgba(8,19,21,0.6) 100%)`,
            backdropFilter: "blur(12px)",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 16,
            boxShadow: hovered
              ? `0 20px 50px ${accentColor}40, inset 0 1px 0 #ffffff08`
              : `0 10px 30px ${accentColor}20, inset 0 1px 0 #ffffff08`,
            overflow: "hidden",
          }}
        >
          {/* Animated glow accent */}
          <div
            style={{
              position: "absolute",
              top: 0,
              left: "10%",
              right: "10%",
              height: 3,
              background: `linear-gradient(90deg, transparent, ${accentColor}, transparent)`,
              borderRadius: 2,
              opacity: hovered ? 1 : 0.6,
              transition: "opacity 0.3s",
            }}
          />

          {/* Icon */}
          <div
            style={{
              width: 72,
              height: 72,
              borderRadius: 18,
              background: `linear-gradient(135deg, ${accentColor}44 0%, ${accentColor}11 100%)`,
              border: `2px solid ${accentColor}55`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 32,
              transition: "transform 0.3s",
              transform: hovered ? "scale(1.1) rotate(5deg)" : "scale(1)",
            }}
          >
            🌐
          </div>

          {/* Title */}
          <div style={{ textAlign: "center" }}>
            <div
              style={{
                fontWeight: 700,
                fontSize: 18,
                color: "#f0f4ff",
                letterSpacing: "-0.01em",
              }}
            >
              X3STAR Website
            </div>
            <div
              style={{
                fontSize: 11,
                color: "#8a95ab",
                marginTop: 4,
                fontFamily: "monospace",
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                maxWidth: 260,
              }}
            >
              Explore ecosystem, governance, capital &amp; market
            </div>
          </div>

          {/* CTA */}
          <div
            style={{
              fontSize: 12,
              color: accentColor,
              fontFamily: "monospace",
              fontWeight: 600,
              marginTop: 8,
              opacity: hovered ? 1 : 0.7,
              transition: "opacity 0.3s",
            }}
          >
            Click to visit →
          </div>

          {/* Ambient background animation */}
          <div
            style={{
              position: "absolute",
              inset: 0,
              borderRadius: 16,
              background: `radial-gradient(circle at 30% 30%, ${accentColor}08 0%, transparent 60%)`,
              pointerEvents: "none",
              opacity: hovered ? 0.8 : 0.3,
              transition: "opacity 0.3s",
            }}
          />

          {/* Shine effect */}
          <div
            style={{
              position: "absolute",
              inset: 0,
              borderRadius: 16,
              background:
                "linear-gradient(135deg, rgba(255,255,255,0.04) 0%, transparent 50%)",
              pointerEvents: "none",
            }}
          />
        </div>
      </div>
    </div>
  );
};

export default React.memo(WebsitePortalCard);
