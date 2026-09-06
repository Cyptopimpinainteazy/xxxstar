import { useRef } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';

/**
 * CenterEye — a sculptural 3D eyeball that presides over the combat arena.
 *
 * The eye floats in the middle of the arena and its iris + pupil track the
 * pointer in 3D space, so it appears to "watch" the mouse wherever it moves.
 *
 * Rendering approach (handles OrbitControls freely rotating the camera):
 *   1. Fire a ray from the camera through the current pointer position.
 *   2. Intersect it with an invisible plane that passes through the eye and
 *      faces the camera — giving the exact 3D point the user is pointing at.
 *   3. Orient a pivot behind the iris so the optic system (iris + pupil) looks
 *      at that point; gaze is clamped so the pupil stays in the aperture.
 *   4. A gentle idle float keeps the eye alive when the mouse is still.
 */
export function CenterEye() {
  const eyeGroup = useRef<THREE.Group>(null!);
  const optic = useRef<THREE.Group>(null!);
  const { camera } = useThree();

  useFrame((state, delta) => {
    if (!optic.current || !eyeGroup.current) return;

    // Build a look target from the pointer ray onto a camera-facing plane at
    // the eye's origin. This keeps the parallax correct as the orbit camera moves.
    const eyePos = eyeGroup.current.getWorldPosition(new THREE.Vector3());
    const planeNormal = camera.getWorldDirection(new THREE.Vector3());
    const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(planeNormal, eyePos);

    const target = new THREE.Vector3();
    camera.updateMatrixWorld();
    camera.parent?.updateMatrixWorld();
    state.raycaster.setFromCamera(state.pointer, camera);
    state.raycaster.ray.intersectPlane(plane, target);

    // Direction from the eye's optical center toward the pointer target.
    const lookDir = target.sub(eyePos);

    // Rotate only far enough that the protruding iris core stays seated on
    // the sclera face (looks like a rolling eye, not a detached ball).
    const maxAbsCos = Math.PI * 0.2; // ~36° full-swing gaze cone
    const azim = THREE.MathUtils.clamp(Math.atan2(lookDir.x, -lookDir.z), -maxAbsCos, maxAbsCos);
    const elev = THREE.MathUtils.clamp(
      Math.asin(THREE.MathUtils.clamp(lookDir.y / Math.max(1e-4, lookDir.length()), -1, 1)),
      -maxAbsCos * 0.8,
      maxAbsCos * 0.8,
    );

    // Smoothly follow the pointer instead of snapping.
    const k = 1 - Math.pow(0.0001, delta);
    // optic.current.rotation ease toward target:
    optic.current.rotation.y += (azim * 0.9 - optic.current.rotation.y) * k;
    optic.current.rotation.x += (elev * 0.9 - optic.current.rotation.x) * k;

    // Gentle idle float so the eye feels alive even when the mouse is still.
    const t = state.clock.elapsedTime;
    eyeGroup.current.position.y = Math.sin(t * 0.6) * 0.15;
    eyeGroup.current.rotation.y = Math.sin(t * 0.25) * 0.08;
  });

  return (
    // Root sits above the agent ring (agents live on the y≈0 plane at radius
    // 4-6) so the overseer eye is clearly central without occluding them.
    <group position={[0, 3.5, 0]}>
      <group ref={eyeGroup}>
        {/* Sclera (white of the eye) — specular node material */}
        <mesh castShadow>
          <sphereGeometry args={[3.4, 64, 64]} />
          <meshPhysicalMaterial
            color="#f4f6ff"
            roughness={0.15}
            metalness={0.0}
            clearcoat={0.9}
            clearcoatRoughness={0.2}
            envMapIntensity={0.8}
          />
        </mesh>

        {/* Optic system (iris core + pupil) — pivots about the eye centre to
            track the pointer. The iris protrudes from the sclera face so it
            stays fully visible across the whole gaze range (no occlusion). */}
        <group ref={optic} position={[0, 0, 0]}>
          {/* Glossy iris lens bulging from the sclera front */}
          <mesh position={[0, 0, 3.0]} castShadow>
            <sphereGeometry args={[2.0, 48, 48]} />
            <meshPhysicalMaterial
              color="#0f2a44"
              roughness={0.1}
              metalness={0.35}
              clearcoat={1}
              clearcoatRoughness={0.15}
              envMapIntensity={1.2}
            />
          </mesh>

          {/* Iris ring — vivid radial bands on the lens cone face */}
          <mesh position={[0, 0, 3.0]} rotation-x={Math.PI / 2}>
            <ringGeometry args={[1.15, 1.85, 64]} />
            <meshStandardMaterial
              color="#1a6fd0"
              roughness={0.2}
              metalness={0.4}
              emissive="#0a5cb8"
              emissiveIntensity={0.5}
              side={THREE.DoubleSide}
            />
          </mesh>
          <mesh position={[0, 0, 3.01]} rotation-x={Math.PI / 2}>
            <ringGeometry args={[1.45, 1.75, 64]} />
            <meshStandardMaterial
              color="#35c3ff"
              roughness={0.3}
              emissive="#29b6f6"
              emissiveIntensity={0.4}
              side={THREE.DoubleSide}
            />
          </mesh>

          {/* Dark pupil on the lens apex */}
          <mesh position={[0, 0, 3.15]}>
            <sphereGeometry args={[0.85, 40, 40]} />
            <meshStandardMaterial
              color="#02060c"
              roughness={0.05}
              metalness={0.6}
            />
          </mesh>

          {/* Specular catch-light that rides with the gaze */}
          <mesh position={[0.55, 0.7, 3.35]}>
            <circleGeometry args={[0.28, 32]} />
            <meshBasicMaterial
              color="#ffffff"
              transparent
              opacity={0.85}
              side={THREE.DoubleSide}
            />
          </mesh>
        </group>
      </group>
    </group>
  );
}

export default CenterEye;
