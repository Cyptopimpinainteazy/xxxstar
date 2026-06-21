import { useEffect, useRef } from 'react'
import * as THREE from 'three'

export default function ThreeBg() {
  const mountRef = useRef<HTMLDivElement>(null)
  const mouseRef = useRef({ x: 0, y: 0 })

  useEffect(() => {
    const container = mountRef.current
    if (!container) return

    // ── Scene ─────────────────────────────────────────────────────────
    const scene = new THREE.Scene()

    // ── Camera ────────────────────────────────────────────────────────
    const camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 0.1, 1500)
    camera.position.set(0, 0, 320)

    // ── Renderer ──────────────────────────────────────────────────────
    const renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: 'low-power',
    })
    renderer.setSize(window.innerWidth, window.innerHeight)
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    container.appendChild(renderer.domElement)

    // ── Star Field ────────────────────────────────────────────────────
    const starCount = 2500
    const starPos = new Float32Array(starCount * 3)
    const starSizes = new Float32Array(starCount)
    const starColors = new Float32Array(starCount * 3)

    for (let i = 0; i < starCount; i++) {
      const r = 100 + Math.random() * 700
      const theta = Math.random() * Math.PI * 2
      const phi = Math.acos(2 * Math.random() - 1)
      starPos[i * 3] = r * Math.sin(phi) * Math.cos(theta)
      starPos[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta)
      starPos[i * 3 + 2] = r * Math.cos(phi)
      starSizes[i] = 0.3 + Math.random() * 2.5

      const colorChoice = Math.random()
      if (colorChoice < 0.6) {
        starColors[i * 3] = 0.8 + Math.random() * 0.2
        starColors[i * 3 + 1] = 0.85 + Math.random() * 0.15
        starColors[i * 3 + 2] = 1.0
      } else if (colorChoice < 0.8) {
        starColors[i * 3] = 0.5 + Math.random() * 0.3
        starColors[i * 3 + 1] = 0.7 + Math.random() * 0.3
        starColors[i * 3 + 2] = 1.0
      } else {
        starColors[i * 3] = 1.0
        starColors[i * 3 + 1] = 0.6 + Math.random() * 0.4
        starColors[i * 3 + 2] = 0.8 + Math.random() * 0.2
      }
    }

    const starGeo = new THREE.BufferGeometry()
    starGeo.setAttribute('position', new THREE.BufferAttribute(starPos, 3))
    starGeo.setAttribute('size', new THREE.BufferAttribute(starSizes, 1))
    starGeo.setAttribute('color', new THREE.BufferAttribute(starColors, 3))

    const starMat = new THREE.PointsMaterial({
      size: 1.2,
      vertexColors: true,
      transparent: true,
      opacity: 0.85,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      sizeAttenuation: true,
    })
    const stars = new THREE.Points(starGeo, starMat)
    scene.add(stars)

    // ── Galaxy Disk ──────────────────────────────────────────────────
    const diskCount = 4000
    const diskPos = new Float32Array(diskCount * 3)
    const diskSizes = new Float32Array(diskCount)
    for (let i = 0; i < diskCount; i++) {
      const angle = Math.random() * Math.PI * 2
      const radius = 50 + Math.pow(Math.random(), 1.5) * 350
      const spread = (Math.random() - 0.5) * 25
      diskPos[i * 3] = Math.cos(angle) * radius
      diskPos[i * 3 + 1] = spread
      diskPos[i * 3 + 2] = Math.sin(angle) * radius
      diskSizes[i] = 0.1 + Math.random() * 0.6
    }

    const diskGeo = new THREE.BufferGeometry()
    diskGeo.setAttribute('position', new THREE.BufferAttribute(diskPos, 3))
    diskGeo.setAttribute('size', new THREE.BufferAttribute(diskSizes, 1))

    const diskMat = new THREE.PointsMaterial({
      size: 0.5,
      color: new THREE.Color(0x4488ff),
      transparent: true,
      opacity: 0.4,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      sizeAttenuation: true,
    })
    const disk = new THREE.Points(diskGeo, diskMat)
    disk.rotation.x = Math.PI / 4
    scene.add(disk)

    // ── Orbital Rings ─────────────────────────────────────────────────
    const rings: { mesh: THREE.Mesh; rotationSpeed: number; angle: number }[] = []

    const createRing = (radius: number, tube: number, color: THREE.Color, opacity: number) => {
      const geo = new THREE.TorusGeometry(radius, tube, 32, 120)
      const mat = new THREE.MeshBasicMaterial({
        color,
        transparent: true,
        opacity,
        side: THREE.DoubleSide,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      })
      const mesh = new THREE.Mesh(geo, mat)
      return mesh
    }

    const ring1 = createRing(100, 0.4, new THREE.Color(0x00c8ff), 0.15)
    ring1.rotation.x = Math.PI / 3.5
    ring1.rotation.z = 0.3
    scene.add(ring1)
    rings.push({ mesh: ring1, rotationSpeed: 0.005, angle: 0 })

    const ring2 = createRing(150, 0.3, new THREE.Color(0x8b5cf6), 0.1)
    ring2.rotation.x = Math.PI / 2.2
    ring2.rotation.z = -0.5
    scene.add(ring2)
    rings.push({ mesh: ring2, rotationSpeed: -0.003, angle: 0 })

    const ring3 = createRing(200, 0.2, new THREE.Color(0x00c8ff), 0.07)
    ring3.rotation.x = Math.PI / 2.8
    ring3.rotation.z = 0.8
    scene.add(ring3)
    rings.push({ mesh: ring3, rotationSpeed: 0.004, angle: 0 })

    // ── Central Glow ──────────────────────────────────────────────────
    const glowGeo = new THREE.IcosahedronGeometry(8, 1)
    const glowMat = new THREE.MeshBasicMaterial({
      color: 0x00aaff,
      transparent: true,
      opacity: 0.3,
      wireframe: true,
    })
    const glow = new THREE.Mesh(glowGeo, glowMat)
    scene.add(glow)

    const innerGeo = new THREE.IcosahedronGeometry(4, 0)
    const innerMat = new THREE.MeshBasicMaterial({
      color: 0x00ddff,
      transparent: true,
      opacity: 0.6,
    })
    const inner = new THREE.Mesh(innerGeo, innerMat)
    scene.add(inner)

    // ── Constellation Lines ───────────────────────────────────────────
    const connectCount = 80
    const conPositions: number[] = []
    for (let i = 0; i < connectCount; i++) {
      const idx1 = Math.floor(Math.random() * starCount)
      const idx2 = Math.floor(Math.random() * starCount)
      const i3_1 = idx1 * 3
      const i3_2 = idx2 * 3
      const dx = starPos[i3_1] - starPos[i3_2]
      const dy = starPos[i3_1 + 1] - starPos[i3_2 + 1]
      const dz = starPos[i3_1 + 2] - starPos[i3_2 + 2]
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz)
      if (dist < 100) {
        conPositions.push(starPos[i3_1], starPos[i3_1 + 1], starPos[i3_1 + 2])
        conPositions.push(starPos[i3_2], starPos[i3_2 + 1], starPos[i3_2 + 2])
      }
    }
    const conGeo = new THREE.BufferGeometry()
    conGeo.setAttribute('position', new THREE.Float32BufferAttribute(conPositions, 3))
    const conMat = new THREE.LineBasicMaterial({
      color: 0x4488ff,
      transparent: true,
      opacity: 0.04,
    })
    const lines = new THREE.LineSegments(conGeo, conMat)
    scene.add(lines)

    // ── Mouse ─────────────────────────────────────────────────────────
    const handleMouse = (e: MouseEvent) => {
      mouseRef.current.x = (e.clientX / window.innerWidth - 0.5) * 2
      mouseRef.current.y = (e.clientY / window.innerHeight - 0.5) * 2
    }
    window.addEventListener('mousemove', handleMouse)

    // ── Resize ────────────────────────────────────────────────────────
    const handleResize = () => {
      camera.aspect = window.innerWidth / window.innerHeight
      camera.updateProjectionMatrix()
      renderer.setSize(window.innerWidth, window.innerHeight)
    }
    window.addEventListener('resize', handleResize)

    // ── Animation ─────────────────────────────────────────────────────
    const clock = new THREE.Clock()

    const animate = () => {
      const elapsed = clock.getElapsedTime()
      const mx = mouseRef.current.x
      const my = mouseRef.current.y

      // Camera parallax
      camera.position.x = mx * 15
      camera.position.y = -my * 10
      camera.lookAt(0, 0, 0)

      // Rotate star field slowly
      stars.rotation.y = elapsed * 0.008
      stars.rotation.x = Math.sin(elapsed * 0.003) * 0.05

      // Rotate galaxy disk
      disk.rotation.y = elapsed * 0.015
      disk.rotation.z = Math.sin(elapsed * 0.01) * 0.02

      // Rotate rings
      for (const ring of rings) {
        ring.mesh.rotation.y += ring.rotationSpeed
      }

      // Pulse central glow
      const pulse = 1 + Math.sin(elapsed * 0.8) * 0.15
      glow.scale.set(pulse, pulse, pulse)
      glow.rotation.x = elapsed * 0.2
      glow.rotation.y = elapsed * 0.3

      inner.rotation.x = -elapsed * 0.15
      inner.rotation.y = elapsed * 0.25

      // Constellation lines fade
      lines.rotation.y = stars.rotation.y
      lines.rotation.x = stars.rotation.x

      renderer.render(scene, camera)
      requestAnimationFrame(animate)
    }

    animate()

    // ── Cleanup ───────────────────────────────────────────────────────
    return () => {
      window.removeEventListener('mousemove', handleMouse)
      window.removeEventListener('resize', handleResize)
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement)
      }
      renderer.dispose()
      starGeo.dispose()
      starMat.dispose()
      diskGeo.dispose()
      diskMat.dispose()
      conGeo.dispose()
      conMat.dispose()
      glowGeo.dispose()
      glowMat.dispose()
      innerGeo.dispose()
      innerMat.dispose()
    }
  }, [])

  return (
    <div
      ref={mountRef}
      className="three-bg"
    />
  )
}
