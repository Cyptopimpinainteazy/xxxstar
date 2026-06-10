import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { useEffect, useRef } from 'react';
import { AgentState } from '../agents/AgentStore';

interface CameraControllerProps {
  selectedAgentId: string | null;
  agents: AgentState[];
}

/**
 * Smooth camera transitions — when an agent is selected,
 * tween the camera to focus on that agent.
 */
export function CameraController({ selectedAgentId, agents }: CameraControllerProps) {
  const { camera } = useThree();
  const targetRef = useRef(new THREE.Vector3(0, 0, 0));
  const currentLookAt = useRef(new THREE.Vector3(0, 0, 0));

  useEffect(() => {
    if (selectedAgentId) {
      const agent = agents.find((a) => a.id === selectedAgentId);
      if (agent) {
        targetRef.current.set(agent.position.x, agent.position.y, agent.position.z);
      }
    } else {
      targetRef.current.set(0, 0, 0);
    }
  }, [selectedAgentId, agents]);

  useFrame((_, delta) => {
    // Smooth lerp toward target
    currentLookAt.current.lerp(targetRef.current, 3.0 * delta);
    camera.lookAt(currentLookAt.current);
  });

  return null;
}