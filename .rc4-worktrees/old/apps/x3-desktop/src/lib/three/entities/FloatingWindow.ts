import * as THREE from 'three';

export interface FloatingWindowConfig {
  width: number;
  height: number;
  title: string;
  color: number;
  iconPath?: string;
}

export class FloatingWindow {
  group: THREE.Group;
  mesh: THREE.Mesh;
  title: string;
  color: number;
  iconPath?: string;

  // Physics
  position: THREE.Vector3 = new THREE.Vector3();
  velocity: THREE.Vector3 = new THREE.Vector3();
  targetPosition: THREE.Vector3 = new THREE.Vector3();
  targetRotation: THREE.Euler = new THREE.Euler();

  // Spring-damping params
  stiffness = 0.15;
  damping = 0.85;
  scale = 1;

  constructor(width: number, height: number, title: string, color: number, iconPath?: string) {
    this.group = new THREE.Group();
    this.title = title;
    this.color = color;
    this.iconPath = iconPath;

    // Create glassmorphic panel
    const geometry = new THREE.PlaneGeometry(width, height);
    const material = new THREE.MeshPhysicalMaterial({
      color: color,
      transmission: 0.6,
      thickness: 0.5,
      roughness: 0.1,
      metalness: 0.2,
      opacity: 0.8,
      transparent: true,
      side: THREE.DoubleSide,
    });

    this.mesh = new THREE.Mesh(geometry, material);
    this.mesh.position.z = 0.1;

    // Add frame/border
    const border = new THREE.EdgesGeometry(geometry);
    const line = new THREE.LineSegments(
      border,
      new THREE.LineBasicMaterial({ color: color, linewidth: 2 })
    );
    this.mesh.add(line);

    this.group.add(this.mesh);

    // Initialize positions
    this.position.copy(this.group.position);
    this.targetPosition.copy(this.position);
  }

  setFocus(focused: boolean) {
    if (this.mesh.material instanceof THREE.MeshPhysicalMaterial) {
      this.mesh.material.opacity = focused ? 1.0 : 0.6;
      this.mesh.material.transmission = focused ? 0.8 : 0.6;
    }
  }

  update(time: number) {
    // Spring physics for position
    const diff = this.targetPosition.clone().sub(this.position);
    this.velocity.lerp(diff.multiplyScalar(this.stiffness), 1 - this.damping);
    this.position.add(this.velocity);

    this.group.position.copy(this.position);

    // Smooth rotation towards target
    this.group.quaternion.setFromEuler(this.targetRotation);

    // Ambient float
    const floatY = Math.sin(time * 0.0005) * 0.5;
    this.group.position.y += floatY * 0.01;

    // Gentle bob and sway
    this.scale = 1 + Math.sin(time * 0.0003) * 0.05;
    this.mesh.scale.set(this.scale, this.scale, 1);
  }

  drawUI(title: string, status: string, data: unknown) {
    // This will be enhanced in React component layer
    console.log(`[${title}] ${status}:`, data);
  }
}
