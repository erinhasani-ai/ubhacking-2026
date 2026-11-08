// Campus map (T32/T33): OpenStreetMap buildings extruded to their tagged or estimated height, roads, water,
// twelve simulated Drones and the simulated fire. Three.js only; the HUD never reads this file.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';

const ACCENT = 0xe3c07c; // champagne, the UI accent
const FIRE = 0xff6a2c;
const GROUND = 0x06080d;
const LABELLED = ['Davis Hall', 'Capen Hall', 'Lockwood Memorial Library', 'Student Union', 'Norton Hall', 'Natural Sciences Complex', 'Center for the Arts', 'Furnas Hall', "O'Brian Hall"];

type XY = [number, number];
/** Map metres (x east, y north, z up) to three.js (Y up, −Z north). */
const v3 = (x: number, y: number, z: number) => new THREE.Vector3(x, z, -y);

export interface CampusHandlers {
  pick(p: XY): void;
  frame(dt: number): void;
}

function dotTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 64;
  const g = c.getContext('2d')!;
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, 'rgba(255,255,255,1)');
  grad.addColorStop(0.35, 'rgba(255,255,255,0.55)');
  grad.addColorStop(1, 'rgba(255,255,255,0)');
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}

export function createCampusScene(canvas: HTMLCanvasElement, overlay: HTMLElement, world: any, data: any, sim: any, on: CampusHandlers, reducedMotion: () => boolean) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: 'high-performance' });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.25)); // measured on the Intel laptop: 26 fps at 2x, 39 at 1.5x
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(GROUND);
  scene.fog = new THREE.Fog(GROUND, 1900, 4600);
  const camera = new THREE.PerspectiveCamera(40, 1, 5, 9000);
  const [pcx, pcy] = world.centre;
  const controls = new OrbitControls(camera, canvas);
  controls.target.copy(v3(pcx, pcy, 0));
  camera.position.copy(v3(pcx + 200, pcy - 1300, 950));
  controls.enableDamping = true;
  controls.screenSpacePanning = false;
  controls.maxPolarAngle = 1.32;
  controls.minDistance = 120;
  controls.maxDistance = 3400;

  scene.add(new THREE.HemisphereLight(0xcfe6ff, 0x08121c, 1.2));
  const sun = new THREE.DirectionalLight(0xffffff, 1.1);
  sun.position.set(-700, 1000, 500);
  scene.add(sun);

  scene.add(new THREE.Mesh(new THREE.PlaneGeometry(9000, 9000).rotateX(-Math.PI / 2).translate(0, -0.5, 0), new THREE.MeshBasicMaterial({ color: 0x0a1520 })));

  const shapeOf = (ring: XY[]) => {
    const pts = ring.slice();
    const [a, b] = [pts[0], pts[pts.length - 1]];
    if (a[0] === b[0] && a[1] === b[1]) pts.pop();
    return new THREE.Shape(pts.map(([x, y]) => new THREE.Vector2(x, y)));
  };

  // Water.
  const waterGeoms = data.water.map((w: any) => new THREE.ShapeGeometry(shapeOf(w.ring)).rotateX(-Math.PI / 2).translate(0, 0.05, 0));
  if (waterGeoms.length) scene.add(new THREE.Mesh(mergeGeometries(waterGeoms), new THREE.MeshBasicMaterial({ color: 0x0e2d4a })));

  // Roads and paths.
  const segs: Record<string, number[]> = { major: [], minor: [], path: [] };
  for (const r of data.roads) for (let i = 1; i < r.pts.length; i++) segs[r.k].push(r.pts[i - 1][0], 0.3, -r.pts[i - 1][1], r.pts[i][0], 0.3, -r.pts[i][1]);
  for (const [k, opacity] of [['major', 0.55], ['minor', 0.3], ['path', 0.13]] as const) {
    const g = new THREE.BufferGeometry().setAttribute('position', new THREE.Float32BufferAttribute(segs[k], 3));
    scene.add(new THREE.LineSegments(g, new THREE.LineBasicMaterial({ color: 0x7ea4c6, transparent: true, opacity, depthWrite: false })));
  }

  // Buildings: one merged mesh for tagged heights, one for estimated (fainter edges).
  const geoms: THREE.BufferGeometry[] = world.buildings.map((b: any) => new THREE.ExtrudeGeometry(shapeOf(b.ring), { depth: b.h, bevelEnabled: false }).rotateX(-Math.PI / 2));
  const pickable: THREE.Mesh[] = [];
  for (const est of [false, true]) {
    const merged = mergeGeometries(world.buildings.filter((b: any) => b.est === est).map((b: any) => geoms[b.i]));
    const mesh = new THREE.Mesh(merged, new THREE.MeshLambertMaterial({ color: est ? 0x112a3f : 0x173650, emissive: 0x0a1d2e }));
    scene.add(mesh);
    pickable.push(mesh);
    scene.add(new THREE.LineSegments(new THREE.EdgesGeometry(merged, 25), new THREE.LineBasicMaterial({ color: ACCENT, transparent: true, opacity: est ? 0.2 : 0.5, depthWrite: false })));
  }

  // Fire highlights: buildings in the alert radius, buildings alerted by a Drone, the burning building.
  const highlight = (opacity: number, color = FIRE) => {
    const m = new THREE.Mesh(new THREE.BufferGeometry(), new THREE.MeshBasicMaterial({ color, transparent: true, opacity, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -1, polygonOffsetUnits: -2 }));
    m.visible = false;
    scene.add(m);
    return m;
  };
  const zoneMesh = highlight(0.2);
  const alertedMesh = highlight(0.5, 0xffa25a);
  const burningMesh = highlight(0.85, 0xff4a1c);
  const setHighlight = (m: THREE.Mesh, idx: number[]) => {
    m.geometry.dispose();
    m.visible = idx.length > 0;
    m.geometry = idx.length ? mergeGeometries(idx.map((i) => geoms[i])) : new THREE.BufferGeometry();
  };

  // One pad per team, spread across campus.
  const padMat = new THREE.MeshBasicMaterial({ color: ACCENT, transparent: true, opacity: 0.6 });
  for (const p of world.pads) {
    const pad = new THREE.Mesh(new THREE.RingGeometry(20, 22, 64).rotateX(-Math.PI / 2), padMat);
    pad.position.copy(v3(p.at[0], p.at[1], 0.6));
    scene.add(pad);
  }

  // Drones: a solid marker, a glow that keeps them visible from 2 km, and a tether to the ground for height.
  const n = sim.drones.length;
  const markers = new THREE.InstancedMesh(new THREE.OctahedronGeometry(4).scale(1, 0.5, 1), new THREE.MeshBasicMaterial({ color: 0xe8f5ff }), n);
  // Alert Drones amber, suppression Drones white.
  sim.drones.forEach((d: any, i: number) => markers.setColorAt(i, new THREE.Color(d.role === 'alert' ? 0xffb35c : 0xe8f5ff)));
  scene.add(markers);
  const glowPos = new Float32Array(n * 3);
  const glow = new THREE.Points(
    new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(glowPos, 3)),
    new THREE.PointsMaterial({ color: ACCENT, size: 26, sizeAttenuation: false, map: dotTexture(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  glow.frustumCulled = false;
  scene.add(glow);
  const tetherPos = new Float32Array(n * 6);
  const tethers = new THREE.LineSegments(new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(tetherPos, 3)), new THREE.LineBasicMaterial({ color: ACCENT, transparent: true, opacity: 0.25 }));
  tethers.frustumCulled = false;
  scene.add(tethers);
  const sprayPos = new Float32Array(n * 6);
  const spray = new THREE.LineSegments(new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(sprayPos, 3)), new THREE.LineBasicMaterial({ color: 0xd9f0ff, transparent: true, opacity: 0.75 }));
  spray.frustumCulled = false;
  scene.add(spray);

  // Fire: burning area (radial glow), alert radius (fill and ring), flames and smoke.
  const burnMat = new THREE.ShaderMaterial({
    uniforms: { uI: { value: 0 }, uT: { value: 0 } },
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    vertexShader: 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }',
    fragmentShader:
      'uniform float uI; uniform float uT; varying vec2 vUv; void main(){ float d = length(vUv - 0.5) * 2.0; float a = smoothstep(1.0, 0.0, d) * uI; float f = 0.88 + 0.12 * sin(uT * 6.0 + d * 14.0); vec3 c = mix(vec3(1.0, 0.25, 0.05), vec3(1.0, 0.75, 0.3), 1.0 - d); gl_FragColor = vec4(c * f, a * f); }',
  });
  const burn = new THREE.Mesh(new THREE.CircleGeometry(1, 64).rotateX(-Math.PI / 2), burnMat);
  const zoneFill = new THREE.Mesh(new THREE.CircleGeometry(1, 96).rotateX(-Math.PI / 2), new THREE.MeshBasicMaterial({ color: FIRE, transparent: true, opacity: 0.07, depthWrite: false }));
  const ringPts = Array.from({ length: 129 }, (_, i) => new THREE.Vector3(Math.cos((i / 128) * Math.PI * 2), 0, Math.sin((i / 128) * Math.PI * 2)));
  const zoneRing = new THREE.Line(new THREE.BufferGeometry().setFromPoints(ringPts), new THREE.LineBasicMaterial({ color: FIRE, transparent: true, opacity: 0.8 }));
  const P = 160;
  const seeds = Array.from({ length: P }, () => [Math.random(), Math.random() * Math.PI * 2, Math.sqrt(Math.random())]);
  const flamePos = new Float32Array(P * 3);
  const smokePos = new Float32Array(P * 3);
  const flames = new THREE.Points(new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(flamePos, 3)), new THREE.PointsMaterial({ color: 0xff8a3a, size: 9, map: dotTexture(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
  const smoke = new THREE.Points(new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(smokePos, 3)), new THREE.PointsMaterial({ color: 0x7d8792, size: 34, map: dotTexture(), transparent: true, opacity: 0.32, depthWrite: false }));
  const fireGroup = new THREE.Group();
  for (const o of [burn, zoneFill, zoneRing, flames, smoke]) {
    o.frustumCulled = false;
    fireGroup.add(o);
  }
  fireGroup.visible = false;
  scene.add(fireGroup);

  // Data pings: every transmission flashes a tiny ring at its sender for 0.6 s (blue telemetry, amber sensors, orange alarms).
  const PINGS = 512;
  const pingPos = new Float32Array(PINGS * 3);
  const pingAge = new Float32Array(PINGS).fill(1);
  const pingKind = new Float32Array(PINGS);
  const pingGeo = new THREE.BufferGeometry();
  pingGeo.setAttribute('position', new THREE.BufferAttribute(pingPos, 3));
  pingGeo.setAttribute('age', new THREE.BufferAttribute(pingAge, 1));
  pingGeo.setAttribute('kind', new THREE.BufferAttribute(pingKind, 1));
  const pings = new THREE.Points(
    pingGeo,
    new THREE.ShaderMaterial({
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      vertexShader:
        'attribute float age; attribute float kind; varying float vA; varying float vK; void main(){ vA = age; vK = kind; gl_PointSize = age >= 1.0 ? 0.0 : 5.0 + age * 22.0; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }',
      fragmentShader:
        'varying float vA; varying float vK; void main(){ float d = length(gl_PointCoord - 0.5) * 2.0; float ring = smoothstep(0.62, 0.8, d) * (1.0 - smoothstep(0.85, 1.0, d)) + (1.0 - smoothstep(0.0, 0.35, d)) * (1.0 - vA); vec3 c = vK < 0.5 ? vec3(0.35, 0.73, 1.0) : vK < 1.5 ? vec3(1.0, 0.72, 0.36) : vec3(1.0, 0.42, 0.17); gl_FragColor = vec4(c, ring * (1.0 - vA)); }',
    }),
  );
  pings.frustumCulled = false;
  scene.add(pings);
  let nextPing = 0;

  // Accelerometer nodes (SIM): a small amber dot on each sensor building's roof.
  const sensorPos = new Float32Array(world.sensors.flatMap((b: any) => [b.c[0], b.h + 1, -b.c[1]]));
  const sensorDots = new THREE.Points(
    new THREE.BufferGeometry().setAttribute('position', new THREE.BufferAttribute(sensorPos, 3)),
    new THREE.PointsMaterial({ color: 0xffb35c, size: 5, sizeAttenuation: false, transparent: true, opacity: 0.8, depthWrite: false }),
  );
  scene.add(sensorDots);

  // Earthquake: a shock ring sweeping out from the epicentre, buildings coloured by shaking, inspected buildings marked.
  const quakeRing = new THREE.Line(new THREE.BufferGeometry().setFromPoints(ringPts), new THREE.LineBasicMaterial({ color: 0xffb35c, transparent: true, opacity: 0.9 }));
  quakeRing.frustumCulled = false;
  quakeRing.visible = false;
  scene.add(quakeRing);
  const strongMesh = highlight(0.55, 0xff5a2a);
  const moderateMesh = highlight(0.35, 0xffb35c);
  const inspectedMesh = highlight(0.75, 0xe8f5ff);
  let quakeKey = '';

  // HTML labels: key buildings, the base, each team and the fire.
  const label = (cls: string, text = '') => {
    const el = document.createElement('div');
    el.className = `label ${cls}`;
    el.textContent = text;
    overlay.append(el);
    return el;
  };
  const buildingLabels = LABELLED.map((name) => world.places.find((p: any) => p.b.name === name)?.b)
    .filter(Boolean)
    .map((b: any) => ({ el: label('building', b.name), at: v3(b.c[0], b.c[1], b.h + 4) }));
  const padLabels = world.pads.map((p: any) => ({ el: label('base', `${p.team[0].toUpperCase()}${p.team.slice(1)} pad`), at: v3(p.at[0], p.at[1], 2) }));
  const teamLabels = world.pads.map((p: any) => ({ id: p.team, el: label('team'), at: new THREE.Vector3() }));
  const fireLabel = { el: label('fire'), at: new THREE.Vector3() };
  const pickLabel = { el: label('picked'), at: new THREE.Vector3() };
  pickLabel.el.hidden = true;

  const tmp = new THREE.Vector3();
  // Writing the same text every frame still forces layout; write only on change.
  const setText = (el: HTMLElement, s: string) => {
    if (el.textContent !== s) el.textContent = s;
  };
  const place =(l: { el: HTMLElement; at: THREE.Vector3 }, w: number, h: number) => {
    tmp.copy(l.at).project(camera);
    const off = tmp.z > 1 || Math.abs(tmp.x) > 1.2 || Math.abs(tmp.y) > 1.2;
    l.el.style.visibility = off ? 'hidden' : 'visible';
    if (!off) l.el.style.transform = `translate(${((tmp.x + 1) / 2) * w}px, ${((1 - tmp.y) / 2) * h}px) translate(-50%, -100%)`;
  };

  // Picking: a click (not a drag) on a building, else the ground.
  const ray = new THREE.Raycaster();
  const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  let down: [number, number] | null = null;
  canvas.addEventListener('pointerdown', (e) => (down = [e.clientX, e.clientY]));
  canvas.addEventListener('pointerup', (e) => {
    if (!down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 6) return;
    down = null;
    const r = canvas.getBoundingClientRect();
    ray.setFromCamera(new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), camera);
    const hit = ray.intersectObjects(pickable)[0];
    const p = hit ? hit.point.clone().addScaledVector(ray.ray.direction, 0.5) : ray.ray.intersectPlane(groundPlane, new THREE.Vector3());
    if (p) on.pick([p.x, -p.z]);
  });

  let goal: THREE.Vector3 | null = null;
  let zoomGoal = 0;
  const zoomDir = new THREE.Vector3();
  const home = { target: controls.target.clone(), position: camera.position.clone() };
  controls.addEventListener('start', () => ((goal = null), (zoomGoal = 0)));

  function resize() {
    const w = innerWidth;
    const h = innerHeight;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
  addEventListener('resize', resize);
  resize();

  let hlKey = '';
  const m4 = new THREE.Matrix4();
  const q = new THREE.Quaternion();
  const one = new THREE.Vector3(1, 1, 1);
  let last = performance.now();
  let frames = 0;
  let clock = 0;

  renderer.setAnimationLoop(() => {
    const now = performance.now();
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    frames++;
    clock += dt;
    on.frame(dt);

    // Drones.
    let sprays = 0;
    const f = sim.fire;
    sim.drones.forEach((d: any, i: number) => {
      const p = v3(d.pos[0], d.pos[1], d.pos[2] + 1.5);
      q.setFromAxisAngle(tmp.set(0, 1, 0), reducedMotion() ? 0 : clock * 2 + i);
      markers.setMatrixAt(i, m4.compose(p, q, one));
      glowPos.set([p.x, p.y, p.z], i * 3);
      tetherPos.set([p.x, p.y, p.z, p.x, 0.3, p.z], i * 6);
      if (f && f.status !== 'out' && d.task.kind === 'suppress' && Math.hypot(d.pos[0] - f.at[0], d.pos[1] - f.at[1]) < 40 && d.pos[2] > 2) {
        sprayPos.set([p.x, p.y, p.z, f.at[0], f.roof + 0.5, -f.at[1]], sprays * 6);
        sprays++;
      }
    });
    markers.instanceMatrix.needsUpdate = true;
    glow.geometry.attributes.position.needsUpdate = true;
    tethers.geometry.attributes.position.needsUpdate = true;
    spray.geometry.attributes.position.needsUpdate = true;
    spray.geometry.setDrawRange(0, sprays * 2);

    // Fire.
    const live = f && f.status !== 'out';
    fireGroup.visible = !!live;
    const key = f ? `${f.status}|${f.building}|${f.zone.length}|${f.alerted.size}` : '';
    if (key !== hlKey) {
      hlKey = key;
      setHighlight(burningMesh, live ? [f.building] : []);
      setHighlight(zoneMesh, live && f.detected ? f.zone.filter((i: number) => i !== f.building) : []);
      setHighlight(alertedMesh, live ? [...f.alerted] : []);
    }
    if (live) {
      const [fx, fy] = f.at;
      burn.position.set(fx, 0.6, -fy);
      burn.scale.setScalar(f.radius * 1.6);
      burnMat.uniforms.uI.value = 0.35 + f.intensity * 0.65;
      burnMat.uniforms.uT.value = reducedMotion() ? 0 : clock;
      const R = f.radius + 150;
      zoneFill.position.set(fx, 0.25, -fy);
      zoneFill.scale.setScalar(R);
      zoneRing.position.set(fx, 1.2, -fy);
      zoneRing.scale.setScalar(R);
      zoneFill.visible = zoneRing.visible = f.detected;
      const t = reducedMotion() ? 0.5 : clock;
      const shown = Math.round(P * Math.max(0.15, f.intensity));
      for (let i = 0; i < P; i++) {
        const [s, a, rr] = seeds[i];
        const u = (s + t * 0.25) % 1;
        const r = f.radius * 0.5 * rr;
        flamePos.set([fx + Math.cos(a) * r, f.roof + u * 22, -(fy + Math.sin(a) * r)], i * 3);
        const su = (s + t * 0.06) % 1;
        smokePos.set([fx + Math.cos(a) * r * 0.6 + su * 90, f.roof + 10 + su * 150, -(fy + Math.sin(a) * r * 0.6 + su * 25)], i * 3);
      }
      flames.geometry.setDrawRange(0, shown);
      smoke.geometry.setDrawRange(0, shown);
      flames.geometry.attributes.position.needsUpdate = true;
      smoke.geometry.attributes.position.needsUpdate = true;
    }

    // Pings age out; the quake ring sweeps and fades.
    for (let i = 0; i < PINGS; i++) if (pingAge[i] < 1) pingAge[i] = Math.min(1, pingAge[i] + dt / 0.6);
    pingGeo.attributes.age.needsUpdate = true;
    const qk_ = sim.quake;
    const qr = qk_ ? (sim.t - qk_.t0) * 350 : 0;
    quakeRing.visible = !!qk_ && qr < 3200;
    if (qk_ && quakeRing.visible) {
      quakeRing.position.set(qk_.at[0], 1.5, -qk_.at[1]);
      quakeRing.scale.setScalar(Math.max(1, qr));
      (quakeRing.material as THREE.LineBasicMaterial).opacity = 0.9 * (1 - qr / 3200);
    }
    const qkey = qk_ ? `${qk_.detected}|${qk_.inspected.size}|${qk_.status}` : '';
    if (qkey !== quakeKey) {
      quakeKey = qkey;
      const shown = qk_ && qk_.detected ? [...qk_.pga.entries()].filter(([i]: [number, number]) => !qk_.inspected.has(i)) : [];
      setHighlight(strongMesh, shown.filter(([, g]: [number, number]) => g >= 0.2).map(([i]: [number, number]) => i));
      setHighlight(moderateMesh, shown.filter(([, g]: [number, number]) => g >= 0.1 && g < 0.2).map(([i]: [number, number]) => i));
      setHighlight(inspectedMesh, qk_ ? [...qk_.inspected] : []);
    }

    if (goal) {
      const delta = goal.clone().sub(controls.target).multiplyScalar(reducedMotion() ? 1 : 1 - Math.exp(-dt * 2.5));
      controls.target.add(delta);
      camera.position.add(delta);
      if (delta.lengthSq() < 0.01) goal = null;
    }
    if (zoomGoal) {
      const dist = camera.position.distanceTo(controls.target);
      const next = dist + (zoomGoal - dist) * (reducedMotion() ? 1 : 1 - Math.exp(-dt * 12));
      zoomDir.subVectors(camera.position, controls.target).setLength(next);
      camera.position.copy(controls.target).add(zoomDir);
      if (Math.abs(next - zoomGoal) < zoomGoal * 0.002) zoomGoal = 0;
    }
    controls.update();
    renderer.render(scene, camera);

    // Labels.
    const w = innerWidth;
    const h = innerHeight;
    for (const l of buildingLabels) place(l, w, h);
    for (const l of padLabels) place(l, w, h);
    for (const l of teamLabels) {
      const ds = sim.drones.filter((d: any) => d.team === l.id);
      const up = ds.filter((d: any) => d.pos[2] > 2);
      const src = up.length ? up : ds;
      const cx = src.reduce((s: number, d: any) => s + d.pos[0], 0) / src.length;
      const cy = src.reduce((s: number, d: any) => s + d.pos[1], 0) / src.length;
      const cz = Math.max(...src.map((d: any) => d.pos[2]));
      l.at.copy(v3(cx, cy, cz + 14));
      setText(l.el, `${l.id[0].toUpperCase()}${l.id.slice(1)} · ${up.length}/${ds.length}`);
      l.el.style.opacity = up.length ? '1' : '0.55';
      place(l, w, h);
    }
    fireLabel.el.hidden = !live;
    if (live) {
      fireLabel.at.copy(v3(f.at[0], f.at[1], f.roof + 40));
      setText(fireLabel.el, `Fire · ${f.detected ? f.status : 'undetected'}`);
      place(fireLabel, w, h);
    }
    if (!pickLabel.el.hidden) place(pickLabel, w, h);
  });

  return {
    /** One transmission: kind 0 telemetry, 1 sensor, 2 alarm. */
    ping(x: number, y: number, z: number, kind = 0) {
      const i = nextPing;
      nextPing = (nextPing + 1) % PINGS;
      pingPos.set([x, z + 2, -y], i * 3);
      pingAge[i] = reducedMotion() ? 0.5 : 0;
      pingKind[i] = kind;
      pingGeo.attributes.position.needsUpdate = true;
      pingGeo.attributes.kind.needsUpdate = true;
    },
    /** Zooms by a factor of the current distance (below 1 is closer), gliding there; clamped to the controls' range. */
    zoomBy(f: number) {
      const from = zoomGoal || camera.position.distanceTo(controls.target);
      zoomGoal = Math.min(controls.maxDistance, Math.max(controls.minDistance, from * f));
    },
    /** Back to the opening view of the whole campus. */
    overview() {
      goal = null;
      zoomGoal = 0;
      controls.target.copy(home.target);
      camera.position.copy(home.position);
      controls.update();
    },
    focus(xy: XY) {
      goal = v3(xy[0], xy[1], 0);
    },
    showPick(b: any | null) {
      pickLabel.el.hidden = !b;
      if (b) {
        pickLabel.el.textContent = b.name ?? 'Unnamed building';
        pickLabel.at.copy(v3(b.c[0], b.c[1], b.h + 4));
      }
    },
    stats: () => ({ frames, renderer: renderer.info.render }),
  };
}
