import * as THREE from 'three';
import { Config } from '../config';

export class Core {
  group: THREE.Group;
  particles: THREE.Points | null = null;
  time = 0;

  constructor() {
    this.group = new THREE.Group();

    // Create particle cloud
    this.createParticles();

    // Central nucleus glow
    this.createNucleus();
  }


  private createParticles() {
    const particleCount = 200;
    const positions = new Float32Array(particleCount * 3);

    for (let i = 0; i < particleCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.random() * Math.PI;
      const radius = Math.random() * 8 + 2;

      positions[i * 3] = Math.sin(phi) * Math.cos(theta) * radius;
      positions[i * 3 + 1] = Math.cos(phi) * radius;
      positions[i * 3 + 2] = Math.sin(phi) * Math.sin(theta) * radius;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    const material = new THREE.PointsMaterial({
      color: Config.colors.primary,
      size: 0.1,
      sizeAttenuation: true,
      transparent: true,
      opacity: 0.6,
    });

    this.particles = new THREE.Points(geometry, material);
    if (this.particles) {
      this.group.add(this.particles);
    }
  }

  private createNucleus() {
    const geometry = new THREE.IcosahedronGeometry(0.5, 4);
    const material = new THREE.MeshStandardMaterial({
      color: Config.colors.primary,
      emissive: Config.colors.primary,
      emissiveIntensity: 1.2,
      metalness: 0.9,
      roughness: 0.1,
    });

    const nucleus = new THREE.Mesh(geometry, material);
    this.group.add(nucleus);
  }

  update(time: number) {
    this.time = time * 0.0001;

    // Rotate particle cloud
    if (this.particles) {
      this.particles.rotation.y += 0.0003;
    }

    // Pulsing nucleus
    const scale = 1 + Math.sin(this.time) * 0.1;
    this.group.children.forEach((child) => {
      if (child instanceof THREE.Mesh && child.geometry instanceof THREE.IcosahedronGeometry) {
        child.scale.set(scale, scale, scale);
      }
    });
  }
}
