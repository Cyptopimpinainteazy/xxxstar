import { AnimatePresence, motion } from 'framer-motion';
import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import x3Chain from '@/services/x3ChainService';
import clsx from 'clsx';

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
  isReal?: boolean;
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
  { city: 'Sao Paulo', country: 'BR', lat: -23.5505, lng: -46.6333 },
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

const statusToHex = (status: ValidatorNode['status']): number => {
  switch (status) {
    case 'online': return 0x00ff9d;
    case 'syncing': return 0xfacc15;
    case 'offline': return 0xef4444;
    default: return 0x9ca3af;
  }
};

const createMockNodes = (): ValidatorNode[] =>
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

const createConnections = (nodes: ValidatorNode[]): ConnectionLine[] => {
  const result: ConnectionLine[] = [];
  nodes.forEach((node, index) => {
    const connectionCount = Math.floor(Math.random() * 2) + 1;
    for (let i = 0; i < connectionCount; i += 1) {
      const targetIndex = Math.floor(Math.random() * nodes.length);
      if (targetIndex !== index) {
        result.push({
          id: `conn-${index}-${targetIndex}-${i}`,
          from: node,
          to: nodes[targetIndex],
          strength: Math.random(),
        });
      }
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

export default function ValidatorGlobe() {
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const nodeGroupRef = useRef<THREE.Group | null>(null);
  const connectionGroupRef = useRef<THREE.Group | null>(null);

  const [nodes, setNodes] = useState<ValidatorNode[]>(createMockNodes());
  const connections = useMemo(() => createConnections(nodes), [nodes]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [realAuthorities, setRealAuthorities] = useState<string[]>([]);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId]
  );

  const loadAuthorities = useCallback(async () => {
    try {
      const stats = await x3Chain.getNetworkStats();
      if (stats.authorities && stats.authorities.length > 0) {
        setRealAuthorities(stats.authorities);
      }
    } catch (err) {
      console.warn('[ValidatorGlobe] Failed to fetch live authorities:', err);
    }
  }, []);

  useEffect(() => {
    loadAuthorities();
    const iv = setInterval(loadAuthorities, 30000);
    return () => clearInterval(iv);
  }, [loadAuthorities]);

  useEffect(() => {
    if (realAuthorities.length === 0) return;
    setNodes(prev => {
      const updated = [...prev];
      realAuthorities.forEach((addr, i) => {
        if (i < updated.length) {
          updated[i] = { ...updated[i], id: addr, status: 'online', isReal: true, score: 99 };
        }
      });
      return updated;
    });
  }, [realAuthorities]);

  // Initial Scene Setup
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const scene = new THREE.Scene();
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 100);
    camera.position.set(0, 0, 4.4);
    cameraRef.current = camera;

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(container.clientWidth, container.clientHeight);
    rendererRef.current = renderer;
    container.appendChild(renderer.domElement);

    const ambientLight = new THREE.AmbientLight(0x9ac4ff, 0.6);
    const directionalLight = new THREE.DirectionalLight(0x7dd3fc, 1.1);
    directionalLight.position.set(3, 2, 4);
    scene.add(ambientLight, directionalLight);

    const globeWireframe = new THREE.LineSegments(
      new THREE.WireframeGeometry(new THREE.SphereGeometry(GLOBE_RADIUS, 48, 48)),
      new THREE.LineBasicMaterial({ color: 0x38bdf8, transparent: true, opacity: 0.36 })
    );
    scene.add(globeWireframe);

    const nodeGroup = new THREE.Group();
    nodeGroupRef.current = nodeGroup;
    scene.add(nodeGroup);

    const connectionGroup = new THREE.Group();
    connectionGroupRef.current = connectionGroup;
    scene.add(connectionGroup);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;
    controls.autoRotate = true;
    controls.autoRotateSpeed = 0.5;

    const animate = () => {
      controls.update();
      renderer.render(scene, camera);
      requestAnimationFrame(animate);
    };
    animate();

    return () => {
      renderer.dispose();
      container.removeChild(renderer.domElement);
    };
  }, []);

  // Update Visuals when nodes change
  useEffect(() => {
    if (!nodeGroupRef.current || !connectionGroupRef.current) return;
    
    // Clear old children
    while(nodeGroupRef.current.children.length > 0) nodeGroupRef.current.remove(nodeGroupRef.current.children[0]);
    while(connectionGroupRef.current.children.length > 0) connectionGroupRef.current.remove(connectionGroupRef.current.children[0]);

    nodes.forEach(node => {
      const pos = latLngToVector3(node.lat, node.lng, GLOBE_RADIUS);
      const dot = new THREE.Mesh(
        new THREE.SphereGeometry(node.isReal ? 0.03 : 0.015, 8, 8),
        new THREE.MeshBasicMaterial({ color: statusToHex(node.status) })
      );
      dot.position.copy(pos);
      nodeGroupRef.current?.add(dot);
    });

    connections.forEach(conn => {
      const v1 = latLngToVector3(conn.from.lat, conn.from.lng, GLOBE_RADIUS);
      const v2 = latLngToVector3(conn.to.lat, conn.to.lng, GLOBE_RADIUS);
      
      const curve = new THREE.QuadraticBezierCurve3(
        v1,
        v1.clone().lerp(v2, 0.5).multiplyScalar(1.2), // Pull point outwards
        v2
      );
      
      const geometry = new THREE.BufferGeometry().setFromPoints(curve.getPoints(20));
      const line = new THREE.Line(
        geometry,
        new THREE.LineBasicMaterial({ color: 0x38bdf8, transparent: true, opacity: 0.2 * conn.strength })
      );
      connectionGroupRef.current?.add(line);
    });
  }, [nodes, connections]);

  return (
    <div className="relative w-full h-[600px] bg-[#0a0a0f] rounded-2xl overflow-hidden shadow-2xl">
      <div ref={containerRef} className="absolute inset-0 cursor-grab active:cursor-grabbing" />
      
      {/* Node Info Overlay */}
      <AnimatePresence>
        {selectedNode && (
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 20 }}
            className="absolute top-6 right-6 w-72 bg-black/60 backdrop-blur-xl border border-white/10 rounded-2xl p-5"
          >
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-bold truncate pr-4">{selectedNode.id.slice(0, 16)}...</h3>
              <div className={clsx(
                "w-2 h-2 rounded-full shadow-[0_0_8px]",
                selectedNode.status === 'online' ? "bg-green-400 shadow-green-400/50" : "bg-red-400 shadow-red-400/50"
              )} />
            </div>
            
            <div className="space-y-3 text-sm">
              <div className="flex justify-between text-gray-400">
                <span>Location</span>
                <span className="text-white">{selectedNode.city}, {selectedNode.country}</span>
              </div>
              <div className="flex justify-between text-gray-400">
                <span>Uptime</span>
                <span className="text-white">{selectedNode.uptime.toFixed(2)}%</span>
              </div>
              <div className="flex justify-between text-gray-400">
                <span>Blocks</span>
                <span className="text-white font-mono">{selectedNode.blocks.toLocaleString()}</span>
              </div>
              <div className="flex justify-between text-gray-400">
                <span>Reputation</span>
                <span className="text-green-400 font-bold">{selectedNode.score}%</span>
              </div>
            </div>

            <button
              onClick={() => setSelectedNodeId(null)}
              className="mt-6 w-full py-2 bg-white/10 hover:bg-white/20 rounded-lg text-xs font-medium transition-colors"
            >
              Close Details
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="absolute bottom-6 left-6 pointer-events-none">
        <div className="text-xs uppercase tracking-widest text-gray-500 font-bold mb-1">Global Network</div>
        <div className="text-2xl font-bold bg-gradient-to-r from-white to-gray-500 bg-clip-text text-transparent">
          {realAuthorities.length || nodes.length} Node Operators
        </div>
      </div>
    </div>
  );
}
