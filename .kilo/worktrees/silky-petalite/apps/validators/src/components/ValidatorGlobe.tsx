'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

interface ValidatorNode {
  id: string;
  lat: number;
  lng: number;
  city: string;
  country: string;
  status: 'online' | 'syncing' | 'offline';
  score: number;
  blocks: number;
  uptime: number;
  connected: boolean;
}

interface ConnectionLine {
  id: string;
  from: ValidatorNode;
  to: ValidatorNode;
  strength: number;
}

const CITIES: Array<{ city: string; country: string; lat: number; lng: number }> = [
  { city: 'New York', country: 'US', lat: 40.7128, lng: -74.006 },
  { city: 'Los Angeles', country: 'US', lat: 34.0522, lng: -118.2437 },
  { city: 'London', country: 'UK', lat: 51.5074, lng: -0.1278 },
  { city: 'Tokyo', country: 'JP', lat: 35.6762, lng: 139.6503 },
  { city: 'Singapore', country: 'SG', lat: 1.3521, lng: 103.8198 },
  { city: 'Sydney', country: 'AU', lat: -33.8688, lng: 151.2093 },
  { city: 'Frankfurt', country: 'DE', lat: 50.1109, lng: 8.6821 },
  { city: 'São Paulo', country: 'BR', lat: -23.5505, lng: -46.6333 },
  { city: 'Mumbai', country: 'IN', lat: 19.076, lng: 72.8777 },
  { city: 'Dubai', country: 'AE', lat: 25.2048, lng: 55.2708 },
  { city: 'Seoul', country: 'KR', lat: 37.5665, lng: 126.978 },
  { city: 'Toronto', country: 'CA', lat: 43.6532, lng: -79.3832 },
  { city: 'Paris', country: 'FR', lat: 48.8566, lng: 2.3522 },
  { city: 'Amsterdam', country: 'NL', lat: 52.3676, lng: 4.9041 },
  { city: 'Hong Kong', country: 'HK', lat: 22.3193, lng: 114.1694 },
  { city: 'Johannesburg', country: 'ZA', lat: -26.2041, lng: 28.0473 },
  { city: 'Mexico City', country: 'MX', lat: 19.4326, lng: -99.1332 },
  { city: 'Stockholm', country: 'SE', lat: 59.3293, lng: 18.0686 },
  { city: 'Zurich', country: 'CH', lat: 47.3769, lng: 8.5417 },
  { city: 'Shanghai', country: 'CN', lat: 31.2304, lng: 121.4737 },
];

const GLOBE_RADIUS = 1.5;

const statusToCssColor = (status: ValidatorNode['status']): string => {
  switch (status) {
    case 'online':
      return '#00ff9d';
    case 'syncing':
      return '#facc15';
    case 'offline':
      return '#ef4444';
    default:
      return '#9ca3af';
  }
};

const statusToHex = (status: ValidatorNode['status']): number => {
  switch (status) {
    case 'online':
      return 0x00ff9d;
    case 'syncing':
      return 0xfacc15;
    case 'offline':
      return 0xef4444;
    default:
      return 0x9ca3af;
  }
};

const createInitialNodes = (): ValidatorNode[] =>
  CITIES.map((city, index) => ({
    id: `VAL-${city.city.slice(0, 3).toUpperCase()}-${String(index).padStart(3, '0')}`,
    lat: city.lat + (Math.random() - 0.5) * 4,
    lng: city.lng + (Math.random() - 0.5) * 4,
    city: city.city,
    country: city.country,
    status: Math.random() > 0.1 ? 'online' : Math.random() > 0.5 ? 'syncing' : 'offline',
    score: Math.floor(70 + Math.random() * 30),
    blocks: Math.floor(Math.random() * 50000),
    uptime: 95 + Math.random() * 5,
    connected: true,
  }));

const createConnections = (initialNodes: ValidatorNode[]): ConnectionLine[] => {
  const result: ConnectionLine[] = [];

  initialNodes.forEach((node, index) => {
    const connectionCount = Math.floor(Math.random() * 3) + 1;

    for (let i = 0; i < connectionCount; i += 1) {
      const targetIndex = Math.floor(Math.random() * initialNodes.length);

      if (targetIndex === index) {
        continue;
      }

      result.push({
        id: `conn-${index}-${targetIndex}-${i}`,
        from: node,
        to: initialNodes[targetIndex],
        strength: Math.random(),
      });
    }
  });

  return result;
};

const latLngToVector3 = (lat: number, lng: number, radius: number): THREE.Vector3 => {
  const phi = ((90 - lat) * Math.PI) / 180;
  const theta = ((lng + 180) * Math.PI) / 180;

  return new THREE.Vector3(
    -radius * Math.sin(phi) * Math.cos(theta),
    radius * Math.cos(phi),
    radius * Math.sin(phi) * Math.sin(theta)
  );
};

const disposeGroup = (group: THREE.Group): void => {
  while (group.children.length > 0) {
    const child = group.children[0];
    group.remove(child);

    child.traverse((object: THREE.Object3D) => {
      const mesh = object as THREE.Mesh;
      if (mesh.geometry) {
        mesh.geometry.dispose();
      }

      const material = mesh.material as THREE.Material | THREE.Material[] | undefined;
      if (Array.isArray(material)) {
        material.forEach((entry) => entry.dispose());
      } else {
        material?.dispose();
      }
    });
  }
};

export default function ValidatorGlobe() {
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const controlsRef = useRef<OrbitControls | null>(null);
  const frameRef = useRef<number | null>(null);
  const nodeGroupRef = useRef<THREE.Group | null>(null);
  const connectionGroupRef = useRef<THREE.Group | null>(null);
  const raycasterRef = useRef(new THREE.Raycaster());
  const pointerRef = useRef(new THREE.Vector2());
  const nodeMapRef = useRef<Map<string, ValidatorNode>>(new Map());

  const [nodes, setNodes] = useState<ValidatorNode[]>([]);
  const [connections, setConnections] = useState<ConnectionLine[]>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId]
  );

  useEffect(() => {
    const initialNodes = createInitialNodes();
    setNodes(initialNodes);
    setConnections(createConnections(initialNodes));
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      setNodes((previousNodes) =>
        previousNodes.map((node) => {
          const shouldFlipStatus = Math.random() > 0.985;
          const nextStatus = shouldFlipStatus
            ? node.status === 'online'
              ? 'syncing'
              : 'online'
            : node.status;

          return {
            ...node,
            blocks: node.blocks + Math.floor(Math.random() * 10),
            score: Math.min(100, Math.max(60, node.score + (Math.random() - 0.5) * 4)),
            status: nextStatus,
          };
        })
      );
    }, 2000);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const scene = new THREE.Scene();
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 100);
    camera.position.set(0, 0, 4.4);
    cameraRef.current = camera;

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    rendererRef.current = renderer;
    container.appendChild(renderer.domElement);

    const ambientLight = new THREE.AmbientLight(0x9ac4ff, 0.6);
    const directionalLight = new THREE.DirectionalLight(0x7dd3fc, 1.1);
    directionalLight.position.set(3, 2, 4);
    scene.add(ambientLight, directionalLight);

    const globeWireframe = new THREE.LineSegments(
      new THREE.WireframeGeometry(new THREE.SphereGeometry(GLOBE_RADIUS, 48, 48)),
      new THREE.LineBasicMaterial({
        color: 0x38bdf8,
        transparent: true,
        opacity: 0.36,
      })
    );
    scene.add(globeWireframe);

    const atmosphere = new THREE.Mesh(
      new THREE.SphereGeometry(GLOBE_RADIUS * 1.05, 36, 36),
      new THREE.MeshBasicMaterial({
        color: 0x0ea5e9,
        wireframe: true,
        transparent: true,
        opacity: 0.08,
      })
    );
    scene.add(atmosphere);

    const starsGeometry = new THREE.BufferGeometry();
    const starsPositionData = new Float32Array(600 * 3);
    for (let i = 0; i < starsPositionData.length; i += 3) {
      const radius = 8 + Math.random() * 8;
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);

      starsPositionData[i] = radius * Math.sin(phi) * Math.cos(theta);
      starsPositionData[i + 1] = radius * Math.sin(phi) * Math.sin(theta);
      starsPositionData[i + 2] = radius * Math.cos(phi);
    }

    starsGeometry.setAttribute('position', new THREE.BufferAttribute(starsPositionData, 3));
    const stars = new THREE.Points(
      starsGeometry,
      new THREE.PointsMaterial({
        color: 0xffffff,
        size: 0.03,
        transparent: true,
        opacity: 0.45,
      })
    );
    scene.add(stars);

    const nodeGroup = new THREE.Group();
    nodeGroupRef.current = nodeGroup;
    scene.add(nodeGroup);

    const connectionGroup = new THREE.Group();
    connectionGroupRef.current = connectionGroup;
    scene.add(connectionGroup);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.enablePan = false;
    controls.enableZoom = false;
    controls.autoRotate = true;
    controls.autoRotateSpeed = 0.55;
    controls.rotateSpeed = 0.7;
    controls.minPolarAngle = Math.PI * 0.2;
    controls.maxPolarAngle = Math.PI * 0.8;
    controlsRef.current = controls;

    const onControlStart = () => setIsDragging(true);
    const onControlEnd = () => setIsDragging(false);
    controls.addEventListener('start', onControlStart);
    controls.addEventListener('end', onControlEnd);

    const handleResize = () => {
      const nextWidth = container.clientWidth;
      const nextHeight = container.clientHeight;

      camera.aspect = nextWidth / nextHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(nextWidth, nextHeight);
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    };

    const handlePointerDown = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointerRef.current.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointerRef.current.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;

      raycasterRef.current.setFromCamera(pointerRef.current, camera);
      const intersections = raycasterRef.current.intersectObjects(nodeGroup.children, true);

      if (intersections.length === 0) {
        setSelectedNodeId(null);
        return;
      }

      const hit = intersections[0].object;
      const nodeId = hit.userData.nodeId as string | undefined;
      if (nodeId) {
        setSelectedNodeId(nodeId);
      }
    };

    const animate = () => {
      const now = performance.now() * 0.001;

      nodeGroup.children.forEach((object: THREE.Object3D, index: number) => {
        if (!(object instanceof THREE.Mesh)) {
          return;
        }

        const baseScale = object.userData.baseScale as number | undefined;
        if (!baseScale) {
          return;
        }

        const pulse = 1 + Math.sin(now * 2.8 + index) * 0.12;
        object.scale.setScalar(baseScale * pulse);
      });

      connectionGroup.children.forEach((object: THREE.Object3D, index: number) => {
        if (!(object instanceof THREE.Line)) {
          return;
        }

        const material = object.material as THREE.LineBasicMaterial;
        material.opacity = 0.09 + ((Math.sin(now * 2 + index) + 1) / 2) * 0.2;
      });

      controls.update();
      renderer.render(scene, camera);
      frameRef.current = window.requestAnimationFrame(animate);
    };

    window.addEventListener('resize', handleResize);
    renderer.domElement.addEventListener('pointerdown', handlePointerDown);
    animate();

    return () => {
      window.removeEventListener('resize', handleResize);
      renderer.domElement.removeEventListener('pointerdown', handlePointerDown);
      controls.removeEventListener('start', onControlStart);
      controls.removeEventListener('end', onControlEnd);
      controls.dispose();

      if (frameRef.current) {
        window.cancelAnimationFrame(frameRef.current);
      }

      if (nodeGroupRef.current) {
        disposeGroup(nodeGroupRef.current);
      }

      if (connectionGroupRef.current) {
        disposeGroup(connectionGroupRef.current);
      }

      scene.traverse((object: THREE.Object3D) => {
        const mesh = object as THREE.Mesh;
        if (mesh.geometry) {
          mesh.geometry.dispose();
        }

        const material = mesh.material as THREE.Material | THREE.Material[] | undefined;
        if (Array.isArray(material)) {
          material.forEach((entry) => entry.dispose());
        } else {
          material?.dispose();
        }
      });

      renderer.dispose();
      renderer.forceContextLoss();
      container.removeChild(renderer.domElement);

      sceneRef.current = null;
      cameraRef.current = null;
      rendererRef.current = null;
      controlsRef.current = null;
      nodeGroupRef.current = null;
      connectionGroupRef.current = null;
    };
  }, []);

  useEffect(() => {
    const nodeGroup = nodeGroupRef.current;
    const connectionGroup = connectionGroupRef.current;

    if (!nodeGroup || !connectionGroup) {
      return;
    }

    disposeGroup(nodeGroup);
    disposeGroup(connectionGroup);
    nodeMapRef.current.clear();

    nodes.forEach((node) => {
      const color = statusToHex(node.status);
      const surfacePoint = latLngToVector3(node.lat, node.lng, GLOBE_RADIUS + 0.04);

      const glow = new THREE.Mesh(
        new THREE.SphereGeometry(0.055, 14, 14),
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0.22,
        })
      );
      glow.position.copy(surfacePoint);
      glow.userData.nodeId = node.id;
      glow.userData.baseScale = 1.15;
      nodeGroup.add(glow);

      const dot = new THREE.Mesh(
        new THREE.SphereGeometry(0.024, 18, 18),
        new THREE.MeshStandardMaterial({
          color,
          emissive: color,
          emissiveIntensity: 0.45,
          roughness: 0.2,
          metalness: 0.35,
        })
      );
      dot.position.copy(surfacePoint);
      dot.userData.nodeId = node.id;
      dot.userData.baseScale = 1;
      nodeGroup.add(dot);

      nodeMapRef.current.set(node.id, node);
    });

    connections.forEach((connection) => {
      const fromPoint = latLngToVector3(connection.from.lat, connection.from.lng, GLOBE_RADIUS + 0.01);
      const toPoint = latLngToVector3(connection.to.lat, connection.to.lng, GLOBE_RADIUS + 0.01);
      const midPoint = fromPoint
        .clone()
        .add(toPoint)
        .multiplyScalar(0.5)
        .normalize()
        .multiplyScalar(GLOBE_RADIUS + 0.35 + connection.strength * 0.22);

      const curve = new THREE.QuadraticBezierCurve3(fromPoint, midPoint, toPoint);
      const points = curve.getPoints(22);

      const geometry = new THREE.BufferGeometry().setFromPoints(points);
      const line = new THREE.Line(
        geometry,
        new THREE.LineBasicMaterial({
          color: 0x22d3ee,
          transparent: true,
          opacity: 0.16,
        })
      );
      connectionGroup.add(line);
    });
  }, [connections, nodes]);

  return (
    <div
      className={`relative w-full h-[70vh] min-h-[520px] md:h-[700px] bg-gradient-radial from-slate-900 via-slate-950 to-black rounded-3xl overflow-hidden ${
        isDragging ? 'cursor-grabbing' : 'cursor-grab'
      }`}
    >
      <div ref={containerRef} className="absolute inset-0" />

      <AnimatePresence>
        {selectedNode && (
          <motion.div
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 50 }}
            className="absolute top-4 right-4 w-80 bg-slate-900/95 backdrop-blur-xl border border-cyan-500/30 rounded-2xl p-6 shadow-2xl shadow-cyan-500/20"
          >
            <button
              className="absolute top-4 right-4 text-gray-500 hover:text-white"
              onClick={() => setSelectedNodeId(null)}
            >
              ✕
            </button>

            <div className="flex items-center gap-3 mb-4">
              <div className="w-4 h-4 rounded-full" style={{ backgroundColor: statusToCssColor(selectedNode.status) }} />
              <h3 className="font-orbitron font-bold text-white">{selectedNode.id}</h3>
            </div>

            <div className="space-y-3">
              <div className="flex justify-between">
                <span className="text-gray-400">Location</span>
                <span className="text-white font-mono">
                  {selectedNode.city}, {selectedNode.country}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span
                  className={`font-mono uppercase ${
                    selectedNode.status === 'online'
                      ? 'text-green-400'
                      : selectedNode.status === 'syncing'
                        ? 'text-yellow-400'
                        : 'text-red-400'
                  }`}
                >
                  {selectedNode.status}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Score</span>
                <div className="flex items-center gap-2">
                  <div className="w-20 h-2 bg-slate-700 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-cyan-500 to-green-500"
                      style={{ width: `${selectedNode.score}%` }}
                    />
                  </div>
                  <span className="text-cyan-400 font-mono">{Math.round(selectedNode.score)}</span>
                </div>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Blocks Validated</span>
                <span className="text-white font-mono">{selectedNode.blocks.toLocaleString()}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Uptime</span>
                <span className="text-green-400 font-mono">{selectedNode.uptime.toFixed(2)}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Coordinates</span>
                <span className="text-gray-300 font-mono text-sm">
                  {selectedNode.lat.toFixed(4)}, {selectedNode.lng.toFixed(4)}
                </span>
              </div>
            </div>

            <div className="mt-6 pt-4 border-t border-slate-700">
              <button className="w-full py-2 bg-cyan-500/20 border border-cyan-500/30 rounded-lg text-cyan-400 font-mono text-sm hover:bg-cyan-500/30 transition-colors">
                VIEW FULL METRICS →
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="absolute top-4 left-4 bg-slate-900/80 backdrop-blur-xl border border-cyan-500/20 rounded-xl p-4">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
          <span className="text-gray-400 text-sm">LIVE NETWORK</span>
        </div>
        <div className="text-3xl font-orbitron font-bold text-cyan-400">
          {nodes.filter((node) => node.status === 'online').length}
        </div>
        <div className="text-xs text-gray-500 uppercase tracking-widest">Validators Online</div>
      </div>

      <div className="absolute bottom-4 left-4 flex gap-6">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-green-400" />
          <span className="text-gray-400 text-sm">Online</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-yellow-400" />
          <span className="text-gray-400 text-sm">Syncing</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-red-400" />
          <span className="text-gray-400 text-sm">Offline</span>
        </div>
      </div>

      <div className="absolute bottom-4 right-4 text-gray-500 text-sm">Drag to rotate • Click node for details</div>
    </div>
  );
}
