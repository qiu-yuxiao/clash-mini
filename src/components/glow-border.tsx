/**
 * FEAT-003: GlowBorder
 *
 * A 4px inner glow border with:
 * - Dual-piston compressible-gas physics: two color poles (amber-gold + electric-blue)
 *   orbit the border as a conic-gradient, repelling each other when close.
 * - Breathing layer (60%): speed driven by real-time network traffic.
 * - Heartbeat layer (40%): fixed 0.9s pulse, always visible even at zero traffic.
 * - Overall intensity scaled by --vibrancy-factor CSS variable.
 */

import { useEffect, useRef } from 'react'

import { useTrafficData } from '@/hooks/use-traffic-data'
import { useVisibility } from '@/hooks/use-visibility'
import { useThemeMode } from '@/services/states'

// ── Physics constants ─────────────────────────────────────────────────────────
/** Repulsion coefficient: higher = stronger push when poles are close */
const REPULSION_K = 0.0008
/** Random kick magnitude (radians/s) */
const KICK_MAGNITUDE = 0.6
/** Minimum time between kicks per pole (ms) */
const KICK_MIN_INTERVAL = 2000
/** Maximum time between kicks per pole (ms) */
const KICK_MAX_INTERVAL = 7000
/** Max angular speed (rad/s) at full traffic */
const MAX_SPEED = Math.PI  // half circle per second at max traffic
/** Min angular speed (rad/s) at zero traffic */
const MIN_SPEED = (2 * Math.PI) / 9  // ~one circle per 9 seconds

// ── Traffic → speed mapping ───────────────────────────────────────────────────
function trafficToBaseSpeed(bytesPerSec: number): number {
  // 0 B/s → MIN_SPEED, ≥10 MB/s → MAX_SPEED (log scale)
  if (bytesPerSec <= 0) return MIN_SPEED
  const log = Math.log10(Math.max(bytesPerSec, 1))
  const logMax = Math.log10(10_000_000) // 10 MB/s
  const t = Math.min(log / logMax, 1)
  return MIN_SPEED + t * (MAX_SPEED - MIN_SPEED)
}

// ── Heartbeat + breathing brightness ─────────────────────────────────────────
function buildBrightnessKeyframes(): string {
  return `
    @keyframes cm-glow-breathe {
      0%   { opacity: var(--cm-breathe-min, 0.08); }
      50%  { opacity: var(--cm-breathe-max, 0.55); }
      100% { opacity: var(--cm-breathe-min, 0.08); }
    }
    @keyframes cm-glow-heartbeat {
      0%   { opacity: 0; }
      10%  { opacity: var(--cm-hb-peak, 0.3); }
      30%  { opacity: 0.05; }
      50%  { opacity: var(--cm-hb-peak, 0.25); }
      70%  { opacity: 0.02; }
      100% { opacity: 0; }
    }
  `
}

// ── Component ─────────────────────────────────────────────────────────────────
export const GlowBorder = () => {
  const mode = useThemeMode()
  const pageVisible = useVisibility()
  const {
    response: { data: traffic },
  } = useTrafficData({ enabled: pageVisible })

  // Canvas-less animation via CSS vars + conic-gradient on a pseudo-element
  const breatheLayerRef = useRef<HTMLDivElement>(null)
  const heartbeatLayerRef = useRef<HTMLDivElement>(null)
  const gradientLayerRef = useRef<HTMLDivElement>(null)

  // Physics state (mutable refs, not React state — updated every RAF)
  const poleAAngle = useRef(0)           // radians
  const poleBAngle = useRef(Math.PI)     // start opposite
  const poleAVel = useRef(MIN_SPEED * 0.7)
  const poleBVel = useRef(-MIN_SPEED * 0.5)
  const lastTimestamp = useRef<number | null>(null)
  const baseSpeedRef = useRef(MIN_SPEED)
  const rafId = useRef<number | null>(null)

  // Scheduled kick times
  const nextKickA = useRef(Date.now() + KICK_MIN_INTERVAL)
  const nextKickB = useRef(Date.now() + KICK_MIN_INTERVAL * 1.5)

  // Update base speed from traffic
  useEffect(() => {
    const total = (traffic?.up ?? 0) + (traffic?.down ?? 0)
    baseSpeedRef.current = trafficToBaseSpeed(total)

    // Update breathing animation duration via CSS variable
    const cycleSec = (2 * Math.PI) / Math.max(baseSpeedRef.current, MIN_SPEED)
    // Clamp to 2s – 10s
    const clampedSec = Math.max(2, Math.min(10, cycleSec))
    document.documentElement.style.setProperty(
      '--cm-breathe-dur',
      `${clampedSec.toFixed(2)}s`,
    )
  }, [traffic])

  // Physics + gradient animation loop
  useEffect(() => {
    let mounted = true

    function tick(ts: number) {
      if (!mounted) return
      if (lastTimestamp.current === null) lastTimestamp.current = ts
      const dt = Math.min((ts - lastTimestamp.current) / 1000, 0.1) // seconds, cap at 100ms
      lastTimestamp.current = ts

      const now = Date.now()

      // Random kicks
      if (now >= nextKickA.current) {
        const kickDir = Math.random() < 0.5 ? 1 : -1
        poleAVel.current += kickDir * KICK_MAGNITUDE * (0.5 + Math.random())
        nextKickA.current =
          now + KICK_MIN_INTERVAL + Math.random() * (KICK_MAX_INTERVAL - KICK_MIN_INTERVAL)
      }
      if (now >= nextKickB.current) {
        const kickDir = Math.random() < 0.5 ? 1 : -1
        poleBVel.current += kickDir * KICK_MAGNITUDE * (0.5 + Math.random())
        nextKickB.current =
          now + KICK_MIN_INTERVAL + Math.random() * (KICK_MAX_INTERVAL - KICK_MIN_INTERVAL)
      }

      // Repulsion force: angular difference
      let diff = poleBAngle.current - poleAAngle.current
      // Normalize to [-π, π]
      while (diff > Math.PI) diff -= 2 * Math.PI
      while (diff < -Math.PI) diff += 2 * Math.PI

      // Repulsion: push apart (force inversely proportional to |diff|)
      const repulsion = REPULSION_K / Math.max(Math.abs(diff), 0.05)
      const repulsionDir = diff > 0 ? -1 : 1
      poleAVel.current += repulsionDir * repulsion
      poleBVel.current -= repulsionDir * repulsion

      // Damping toward base speed (gentle drag)
      const base = baseSpeedRef.current
      const damping = 0.3
      poleAVel.current += (Math.sign(poleAVel.current) * base - poleAVel.current) * damping * dt
      poleBVel.current += (Math.sign(poleBVel.current) * base - poleBVel.current) * damping * dt

      // Clamp speed
      const maxSpd = MAX_SPEED * 2
      poleAVel.current = Math.max(-maxSpd, Math.min(maxSpd, poleAVel.current))
      poleBVel.current = Math.max(-maxSpd, Math.min(maxSpd, poleBVel.current))

      // Integrate positions
      poleAAngle.current += poleAVel.current * dt
      poleBAngle.current += poleBVel.current * dt

      // Normalize angles to [0, 2π]
      poleAAngle.current = ((poleAAngle.current % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI)
      poleBAngle.current = ((poleBAngle.current % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI)

      // Build conic-gradient
      const aDeg = (poleAAngle.current * 180) / Math.PI
      const bDeg = (poleBAngle.current * 180) / Math.PI

      // Amber-gold at A, electric-blue at B, smooth gradient between
      const gradient = `conic-gradient(from 0deg, 
        #D4AF37 ${aDeg.toFixed(1)}deg,
        #0084FF ${bDeg.toFixed(1)}deg,
        #D4AF37 ${(aDeg + 360).toFixed(1)}deg
      )`

      if (gradientLayerRef.current) {
        gradientLayerRef.current.style.background = gradient
      }

      rafId.current = requestAnimationFrame(tick)
    }

    rafId.current = requestAnimationFrame(tick)
    return () => {
      mounted = false
      if (rafId.current !== null) cancelAnimationFrame(rafId.current)
    }
  }, [])

  // Inject keyframes once
  useEffect(() => {
    const styleId = 'cm-glow-border-keyframes'
    if (!document.getElementById(styleId)) {
      const style = document.createElement('style')
      style.id = styleId
      style.textContent = buildBrightnessKeyframes()
      document.head.appendChild(style)
    }
  }, [])

  const isLight = mode === 'light'

  return (
    <>
      {/* Gradient flow layer: the conic gradient representing the two poles */}
      <div
        ref={gradientLayerRef}
        style={{
          position: 'fixed',
          inset: 0,
          pointerEvents: 'none',
          zIndex: 9998,
          // Mask: only show the outer 4px ring
          WebkitMask:
            'linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)',
          WebkitMaskComposite: 'xor',
          maskComposite: 'exclude',
          padding: '4px',
          borderRadius: '0px',
          // Base opacity driven by breathing animation (will be overridden by layers below)
        }}
      />

      {/* Breathing layer (60% weight): slow pulse driven by traffic speed */}
      <div
        ref={breatheLayerRef}
        style={{
          position: 'fixed',
          inset: 0,
          pointerEvents: 'none',
          zIndex: 9999,
          WebkitMask:
            'linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)',
          WebkitMaskComposite: 'xor',
          maskComposite: 'exclude',
          padding: '4px',
          background: isLight
            ? 'linear-gradient(135deg, #D4AF37, #0084FF)'
            : 'linear-gradient(135deg, #FFD700, #00BFFF)',
          animation: 'cm-glow-breathe var(--cm-breathe-dur, 5s) ease-in-out infinite',
          // Opacity range for breathe layer: 0.05 (dim) to 0.55*0.6=0.33 (bright) → 60% share
          '--cm-breathe-min': '0.04',
          '--cm-breathe-max': '0.33',
          // Box-shadow glow outward
          boxShadow: isLight
            ? 'inset 0 0 8px rgba(212,175,55,0.4), inset 0 0 8px rgba(0,132,255,0.4)'
            : 'inset 0 0 12px rgba(255,215,0,0.5), inset 0 0 12px rgba(0,191,255,0.5)',
          mixBlendMode: 'screen',
        } as React.CSSProperties}
      />

      {/* Heartbeat layer (40% weight): fixed 0.9s pulse, always alive */}
      <div
        ref={heartbeatLayerRef}
        style={{
          position: 'fixed',
          inset: 0,
          pointerEvents: 'none',
          zIndex: 10000,
          WebkitMask:
            'linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)',
          WebkitMaskComposite: 'xor',
          maskComposite: 'exclude',
          padding: '4px',
          background: isLight
            ? 'linear-gradient(45deg, #FFB300, #40C4FF)'
            : 'linear-gradient(45deg, #FFC107, #29B6F6)',
          animation: 'cm-glow-heartbeat 0.9s ease-in-out infinite',
          '--cm-hb-peak': '0.22',  // 40% share peak
          mixBlendMode: 'screen',
        } as React.CSSProperties}
      />
    </>
  )
}
