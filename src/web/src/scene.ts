// The Room Twin and the Swarm in Three.js, updated straight from the stream, never through Svelte state.
// Mission frame NED → Three.js, once: (x, y, z) = (E, −D, −N).
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  Color,
  DoubleSide,
  Group,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  RingGeometry,
  Points,
  Raycaster,
  Scene,
  ShaderMaterial,
  SRGBColorSpace,
  Sprite,
  SpriteMaterial,
  Vector2,
  Vector3,
  Vector4,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { buildPlan } from './plan';
import type { Hello, State } from './stream';

const INK = '#ece7df';
const ACCENT = '#e3c07c'; // champagne: Drones and the active target stand apart from the blue scan
const ACCENT_HOVER = '#eed39f';
const ACCENT_SELECTED = '#fff0cc';
const STALE_OPACITY = 0.4;
const GROUND = '#06080d';

/** Height above the floor above which the scan's ceiling is cut away, like a plan section. */
export const CEILING_CUT_M = 2.35; // Davis third-floor corridors: ceiling about 2.7 m above the floor
const POINT_SIZE_M = 0.048;
const RENDER_DELAY_MS = 50; // render this far behind the newest state frame, so there is a frame to interpolate toward
const MAX_EXTRAPOLATE_MS = 100;
const SNAPSHOTS = 8;
const MAX_DRONES = 16;
const DRONE_HIT_PX = 22; // 44 px target
const CLICK_SLOP_PX = 6;
const GLYPH_M = 0.38;
const LABEL_PX = { w: 26, h: 18 };
const MARKER_PX = 30;

export interface SceneHandlers {
  /** A click on a Drone: select or deselect (shift adds). */
  drone(id: number, shift: boolean): void;
  /** A click on the Room Twin at a Mission-frame point; returns false if the Command could not be sent. */
  spot(ned: { n: number; e: number; d: number }): boolean;
}

// ---------- Small canvas-drawn textures (drawn once) ----------

function canvasTexture(w: number, h: number, draw: (g: CanvasRenderingContext2D) => void) {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  draw(c.getContext('2d')!);
  const t = new CanvasTexture(c);
  t.colorSpace = SRGBColorSpace;
  return t;
}

// A quadrotor seen from above: four rotor rings on an X, a hub, and a heading notch at the front (texture top = forward).
const glyphTexture = () =>
  canvasTexture(256, 256, (g) => {
    g.strokeStyle = '#fff';
    g.fillStyle = '#fff';
    g.lineCap = 'round';
    const c = 128, r = 64, rotor = 30;
    g.lineWidth = 16;
    for (const [sx, sy] of [[-1, -1], [1, -1], [1, 1], [-1, 1]]) {
      g.beginPath();
      g.moveTo(c + sx * 18, c + sy * 18);
      g.lineTo(c + sx * (r - rotor * 0.7), c + sy * (r - rotor * 0.7));
      g.stroke();
      g.beginPath();
      g.arc(c + sx * r, c + sy * r, rotor, 0, Math.PI * 2);
      g.globalAlpha = 0.3; // rotor disc
      g.fill();
      g.globalAlpha = 1;
      g.stroke();
    }
    g.beginPath();
    g.arc(c, c, 22, 0, Math.PI * 2);
    g.fill();
    g.beginPath(); // heading notch
    g.moveTo(c, 4);
    g.lineTo(c - 16, 34);
    g.lineTo(c + 16, 34);
    g.closePath();
    g.fill();
  });

const shadowTexture = () =>
  canvasTexture(128, 128, (g) => {
    const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64);
    grad.addColorStop(0, 'rgba(0,0,0,0.85)');
    grad.addColorStop(0.55, 'rgba(0,0,0,0.35)');
    grad.addColorStop(1, 'rgba(0,0,0,0)');
    g.fillStyle = grad;
    g.fillRect(0, 0, 128, 128);
  });

function labelTexture(id: number, selected: boolean, hover: boolean) {
  const s = 4; // drawn at 4x for crisp text on Retina
  return canvasTexture(LABEL_PX.w * s, LABEL_PX.h * s, (g) => {
    g.scale(s, s);
    g.fillStyle = selected ? ACCENT : hover ? '#3a3732' : 'rgba(20,19,17,0.92)';
    g.beginPath();
    g.roundRect(0.5, 0.5, LABEL_PX.w - 1, LABEL_PX.h - 1, 5);
    g.fill();
    if (!selected) {
      g.strokeStyle = 'rgba(236,231,223,0.28)';
      g.lineWidth = 1;
      g.stroke();
    }
    g.fillStyle = selected ? '#1a0c03' : INK;
    g.font = '600 11px "Geist Mono Variable", ui-monospace, Menlo, monospace';
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    g.fillText(String(id), LABEL_PX.w / 2, LABEL_PX.h / 2 + 0.5);
  });
}

function markerTexture(kind: 'pending' | 'rejected') {
  const s = 4, w = MARKER_PX;
  return canvasTexture(w * s, w * s, (g) => {
    g.scale(s, s);
    const c = w / 2;
    g.lineCap = 'round';
    if (kind === 'pending') {
      g.strokeStyle = 'rgba(12,12,11,0.9)'; // dark halo keeps the ring readable over bright points
      g.lineWidth = 4.5;
      g.beginPath();
      g.arc(c, c, 10, 0, Math.PI * 2);
      g.stroke();
      g.strokeStyle = ACCENT;
      g.lineWidth = 2;
      g.stroke();
      for (const [dx, dy] of [[0, -1], [1, 0], [0, 1], [-1, 0]]) {
        g.beginPath();
        g.moveTo(c + dx * 12.5, c + dy * 12.5);
        g.lineTo(c + dx * 15, c + dy * 15);
        g.stroke();
      }
      g.fillStyle = ACCENT;
      g.beginPath();
      g.arc(c, c, 2.2, 0, Math.PI * 2);
      g.fill();
    } else {
      // Alarm: a triangle with an exclamation mark, in the accent.
      for (const [style, width] of [['rgba(12,12,11,0.9)', 5], [ACCENT, 2.25]] as const) {
        g.strokeStyle = style;
        g.lineWidth = width;
        g.lineJoin = 'round';
        g.beginPath();
        g.moveTo(c, c - 9);
        g.lineTo(c + 9, c + 7);
        g.lineTo(c - 9, c + 7);
        g.closePath();
        g.moveTo(c, c - 3);
        g.lineTo(c, c + 1.5);
        g.moveTo(c, c + 4);
        g.lineTo(c, c + 4.2);
        g.stroke();
      }
    }
  });
}

// ---------- Point cloud material ----------

const pointsMaterial = () =>
  new ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
    uniforms: {
      uSize: { value: POINT_SIZE_M },
      uScale: { value: 1 },
      uHeight: { value: 3 },
      uLow: { value: new Color('#125cdb') },
      uHigh: { value: new Color('#62caff') },
      uGround: { value: new Color(GROUND) },
      uFog: { value: new Vector2(8, 30) },
      uOpacity: { value: 1 },
    },
    vertexShader: /* glsl */ `
      uniform float uSize;
      uniform float uScale;
      uniform float uHeight;
      uniform vec3 uLow;
      uniform vec3 uHigh;
      uniform vec3 uGround;
      uniform vec2 uFog;
      varying vec3 vColor;
      void main() {
        vec4 mv = modelViewMatrix * vec4(position, 1.0);
        gl_Position = projectionMatrix * mv;
        float dist = -mv.z;
        gl_PointSize = clamp(uSize * uScale / dist, 2.0, 9.0);
        // Blue depth tint: furniture and walls stay legible through the translucent cloud.
        float h = clamp(position.y / uHeight, 0.0, 1.0);
        vec3 c = mix(uLow, uHigh, smoothstep(0.0, 0.8, h));
        vColor = mix(c, uGround, 0.6 * smoothstep(uFog.x, uFog.y, dist));
      }`,
    fragmentShader: /* glsl */ `
      uniform float uOpacity;
      varying vec3 vColor;
      void main() {
        vec2 p = gl_PointCoord * 2.0 - 1.0;
        float r2 = dot(p, p);
        if (r2 > 1.0) discard;
        float glow = exp(-3.5 * r2) * (1.0 - smoothstep(0.7, 1.0, r2));
        gl_FragColor = vec4(vColor, 0.38 * glow * uOpacity);
        #include <colorspace_fragment>
      }`,
  });

// Hologram layer (plan.ts): flat glass walls brightening toward a crisp top edge, a flat floor with a 1 m light grid,
// and a slow scan sweep. Additive with depth writes off, so the points and Drones always read through it.
const sweepGlsl = /* glsl */ `
  uniform float uTime;
  uniform vec2 uSweep;
  uniform float uMotion;
  float sweep(float z) {
    float span = uSweep.y - uSweep.x;
    float at = uSweep.x + mod(uTime * 5.0, span + 10.0) - 5.0;
    return uMotion * exp(-pow((z - at) / 0.8, 2.0));
  }`;
const holoUniforms = () => ({
  uTime: { value: 0 },
  uSweep: { value: new Vector2(-30, 30) },
  uMotion: { value: 1 },
  uFog: { value: new Vector2(8, 30) },
  uLow: { value: new Color('#0a4bd6') },
  uHigh: { value: new Color('#59e1ff') },
});
const wallMaterial = (u: ReturnType<typeof holoUniforms>) =>
  new ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
    side: DoubleSide,
    uniforms: u,
    vertexShader: /* glsl */ `
      attribute float aH;
      varying float vH;
      varying float vZ;
      varying float vDist;
      void main() {
        vH = aH;
        vZ = position.z;
        vec4 mv = modelViewMatrix * vec4(position, 1.0);
        vDist = -mv.z;
        gl_Position = projectionMatrix * mv;
      }`,
    fragmentShader: /* glsl */ `
      ${sweepGlsl}
      uniform vec2 uFog;
      uniform vec3 uLow;
      uniform vec3 uHigh;
      varying float vH;
      varying float vZ;
      varying float vDist;
      void main() {
        float b = sweep(vZ);
        float a = 0.05 + 0.32 * pow(vH, 3.0) + 0.3 * b;
        a *= 1.0 - 0.7 * smoothstep(uFog.x, uFog.y, vDist);
        gl_FragColor = vec4(mix(uLow, uHigh, vH) + b * vec3(0.3, 0.5, 0.5), a);
        #include <colorspace_fragment>
      }`,
  });
const floorMaterial = (u: ReturnType<typeof holoUniforms>, mask: CanvasTexture, rect: Vector4) =>
  new ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
    uniforms: { ...u, uMask: { value: mask }, uRect: { value: rect } },
    vertexShader: /* glsl */ `
      varying vec3 vPos;
      varying float vDist;
      void main() {
        vPos = position;
        vec4 mv = modelViewMatrix * vec4(position, 1.0);
        vDist = -mv.z;
        gl_Position = projectionMatrix * mv;
      }`,
    fragmentShader: /* glsl */ `
      ${sweepGlsl}
      uniform vec2 uFog;
      uniform vec3 uLow;
      uniform vec3 uHigh;
      uniform sampler2D uMask;
      uniform vec4 uRect;
      varying vec3 vPos;
      varying float vDist;
      void main() {
        vec2 uv = (vPos.xz - uRect.xy) / uRect.zw;
        float m = smoothstep(0.42, 0.58, texture2D(uMask, uv).r);
        if (m < 0.01) discard;
        vec2 g = abs(fract(vPos.xz - 0.5) - 0.5) / max(fwidth(vPos.xz), vec2(1e-4));
        float grid = 1.0 - min(min(g.x, g.y), 1.0);
        float b = sweep(vPos.z);
        float a = m * (0.06 + 0.2 * grid + 0.25 * b);
        a *= 1.0 - 0.7 * smoothstep(uFog.x, uFog.y, vDist);
        gl_FragColor = vec4(mix(uLow, uHigh, 0.35 + 0.5 * grid) + b * vec3(0.3, 0.5, 0.5), a);
        #include <colorspace_fragment>
      }`,
  });

// ---------- Interpolation buffer per Drone (preallocated; nothing is allocated per frame) ----------

class Track {
  t = new Float64Array(SNAPSHOTS);
  v = new Float32Array(SNAPSHOTS * 6); // n, e, d, roll, pitch, yaw
  vel = new Float32Array(3);
  count = 0;
  head = -1;
  push(tMs: number, s: State['drones'][number]) {
    this.head = (this.head + 1) % SNAPSHOTS;
    this.t[this.head] = tMs;
    const o = this.head * 6;
    this.v[o] = s.n;
    this.v[o + 1] = s.e;
    this.v[o + 2] = s.d;
    this.v[o + 3] = s.roll;
    this.v[o + 4] = s.pitch;
    this.v[o + 5] = s.yaw;
    this.vel[0] = s.vn;
    this.vel[1] = s.ve;
    this.vel[2] = s.vd;
    if (this.count < SNAPSHOTS) this.count++;
  }
  /** Writes the pose at server time rt (ms) into out[0..5]. */
  sample(rt: number, out: Float32Array) {
    let newer = this.head;
    for (let k = 0; k < this.count; k++) {
      const i = (this.head - k + SNAPSHOTS) % SNAPSHOTS;
      if (this.t[i] <= rt) {
        if (k === 0) {
          // Past the newest frame: extrapolate along the reported velocity, briefly.
          const dt = Math.min(rt - this.t[i], MAX_EXTRAPOLATE_MS) / 1000;
          const o = i * 6;
          for (let a = 0; a < 6; a++) out[a] = this.v[o + a];
          out[0] += this.vel[0] * dt;
          out[1] += this.vel[1] * dt;
          out[2] += this.vel[2] * dt;
          return;
        }
        const t0 = this.t[i], t1 = this.t[newer];
        const f = t1 > t0 ? (rt - t0) / (t1 - t0) : 1;
        const o0 = i * 6, o1 = newer * 6;
        for (let a = 0; a < 5; a++) out[a] = this.v[o0 + a] + (this.v[o1 + a] - this.v[o0 + a]) * f;
        let dy = this.v[o1 + 5] - this.v[o0 + 5];
        dy = Math.atan2(Math.sin(dy), Math.cos(dy));
        out[5] = this.v[o0 + 5] + dy * f;
        return;
      }
      newer = i;
    }
    const o = newer * 6; // older than every frame held: the oldest one
    for (let a = 0; a < 6; a++) out[a] = this.v[o + a];
  }
}

interface DroneView {
  id: number;
  track: Track;
  group: Group;
  glyph: Mesh<PlaneGeometry, MeshBasicMaterial>;
  ring: Mesh<RingGeometry, MeshBasicMaterial>; // shown around a selected Drone
  shadow: Mesh<PlaneGeometry, MeshBasicMaterial>;
  label: Sprite;
  tex: { plain: CanvasTexture; hover: CanvasTexture; selected: CanvasTexture };
  pos: Vector3; // interpolated, Three.js coordinates
}

export function createScene(canvas: HTMLCanvasElement, on: SceneHandlers, reducedMotion: () => boolean) {
  // No MSAA: on this laptop's Intel Iris Plus it cost a third of the frame rate at Retina size (39 vs 60 fps, measured).
  const renderer = new WebGLRenderer({ canvas, antialias: false, powerPreference: 'high-performance' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setClearColor(GROUND);

  const scene = new Scene();
  const camera = new PerspectiveCamera(40, 1, 0.05, 200);
  camera.position.set(4, 9, 13);
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.12;
  controls.maxPolarAngle = Math.PI * 0.48; // never below the floor
  controls.minDistance = 1.5;
  controls.maxDistance = 140; // a whole 60 m floor fits
  controls.screenSpacePanning = false; // pan along the floor
  controls.zoomToCursor = true; // scroll toward what is under the pointer, to explore a large floor
  controls.keyPanSpeed = 14;
  controls.listenToKeyEvents(window); // arrow keys pan; shift + arrows orbit

  // Point cloud.
  const pointsGeom = new BufferGeometry();
  const pointsMat = pointsMaterial();
  const cloud = new Points(pointsGeom, pointsMat);
  cloud.frustumCulled = false;
  scene.add(cloud);

  // Hologram layer (from /api/mesh through plan.ts), drawn under the points.
  const holoU = holoUniforms();
  const holo = new Group();
  holo.visible = false;
  scene.add(holo);
  let holoOn = true;

  // Launch pad slots: thin rings on the floor.
  const padLines = new LineSegments(new BufferGeometry(), new LineBasicMaterial({ color: '#8a847b', transparent: true, opacity: 0.7 }));
  scene.add(padLines);

  // Drop lines from each Drone to the ground under it.
  const dropPos = new Float32Array(MAX_DRONES * 6);
  const dropGeom = new BufferGeometry();
  dropGeom.setAttribute('position', new BufferAttribute(dropPos, 3));
  const drops = new LineSegments(dropGeom, new LineBasicMaterial({ color: INK, transparent: true, opacity: 0.32, depthTest: false }));
  drops.renderOrder = 2;
  drops.frustumCulled = false;
  scene.add(drops);

  const glyphTex = glyphTexture();
  const shadowTex = shadowTexture();
  const glyphGeom = new PlaneGeometry(GLYPH_M, GLYPH_M).rotateX(-Math.PI / 2); // flat; texture top points to −z (forward)
  const shadowGeom = new PlaneGeometry(0.46, 0.46).rotateX(-Math.PI / 2);
  const ringGeom = new RingGeometry(0.27, 0.295, 48).rotateX(-Math.PI / 2);
  const drones: DroneView[] = [];
  const byId = new Map<number, DroneView>();

  // Target markers, constant-size billboards: `pending` from pointerdown until its Verdict, `active` at the approved target.
  const markerTex = { pending: markerTexture('pending'), rejected: markerTexture('rejected') };
  const markerSprite = () => {
    const m = new Sprite(new SpriteMaterial({ map: markerTex.pending, depthTest: false, transparent: true, sizeAttenuation: false }));
    m.renderOrder = 4;
    m.visible = false;
    scene.add(m);
    return m;
  };
  const marker = markerSprite();
  const active = markerSprite();
  let markerState: 'provisional' | 'pending' | 'rejected' = 'pending';
  let markerFadeAt = 0;

  const selected = new Set<number>();
  let stale = false;
  let hovered = 0;
  let pressed = 0;
  let follow = false;
  let helloSeen = false;
  let padBox = { n0: 0, n1: 0, e0: 0, e1: 0, d: 0 };
  let clockOffset = NaN; // performance.now() − server t_ms, the smallest seen (least delayed frame)
  let lastT = -Infinity;
  let rightInset = 0;
  let uiScale = 1; // the HUD's rem factor, which also scales the chip
  let labelScale = 0.02;
  let markerScale = 0.02;
  let frames = 0;
  let pointCount = 0;

  const pose = new Float32Array(6);
  const tmp = new Vector3();
  const tmp2 = new Vector3();
  const ndc = new Vector2();
  const raycaster = new Raycaster();
  const followGoal = new Vector3();

  function droneView(id: number): DroneView {
    let v = byId.get(id);
    if (v) return v;
    const group = new Group();
    const glyph = new Mesh(glyphGeom, new MeshBasicMaterial({ map: glyphTex, color: INK, transparent: true, depthTest: false, depthWrite: false, side: DoubleSide }));
    glyph.renderOrder = 3;
    const ring = new Mesh(ringGeom, new MeshBasicMaterial({ color: ACCENT, transparent: true, depthTest: false, depthWrite: false, side: DoubleSide }));
    ring.renderOrder = 3;
    ring.visible = false;
    group.add(glyph, ring);
    const shadow = new Mesh(shadowGeom, new MeshBasicMaterial({ map: shadowTex, transparent: true, depthWrite: false }));
    shadow.renderOrder = 1;
    const tex = { plain: labelTexture(id, false, false), hover: labelTexture(id, false, true), selected: labelTexture(id, true, false) };
    const label = new Sprite(new SpriteMaterial({ map: tex.plain, depthTest: false, transparent: true, sizeAttenuation: false }));
    label.center.set(-0.5, -0.55); // up and to the right of the glyph, a constant pixel offset
    label.scale.set(LABEL_PX.w * labelScale, LABEL_PX.h * labelScale, 1);
    label.renderOrder = 5;
    scene.add(group, shadow, label);
    v = { id, track: new Track(), group, glyph, ring, shadow, label, tex, pos: new Vector3() };
    drones.push(v);
    byId.set(id, v);
    applyLook(v);
    return v;
  }

  function applyLook(v: DroneView) {
    const sel = selected.has(v.id);
    v.glyph.material.color.set(sel ? ACCENT_SELECTED : v.id === hovered ? ACCENT_HOVER : ACCENT);
    v.ring.visible = sel;
    const opacity = stale ? STALE_OPACITY : 1;
    v.glyph.material.opacity = v.ring.material.opacity = v.label.material.opacity = opacity;
    v.label.material.map = sel ? v.tex.selected : v.id === hovered ? v.tex.hover : v.tex.plain;
    v.glyph.scale.setScalar(v.id === pressed ? 0.9 : 1);
  }

  function resize() {
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    // Centre the view in the space left of the Drone list.
    if (rightInset > 0) camera.setViewOffset(w, h, rightInset / 2, 0, w, h);
    else camera.clearViewOffset();
    camera.updateProjectionMatrix();
    const k = 2 * Math.tan((camera.fov * Math.PI) / 360) / h; // sprite scale per CSS pixel (sizeAttenuation off)
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) / 15;
    uiScale = rem;
    labelScale = k * rem;
    markerScale = k * MARKER_PX * rem;
    for (const v of drones) v.label.scale.set(LABEL_PX.w * labelScale, LABEL_PX.h * labelScale, 1);
    marker.scale.set(markerScale, markerScale, 1);
    active.scale.set(markerScale, markerScale, 1);
    pointsMat.uniforms.uScale.value = (h * renderer.getPixelRatio()) / (2 * Math.tan((camera.fov * Math.PI) / 360));
  }
  // Until the operator moves the camera, keep the room framed as the window or the HUD changes size.
  let framed: { min: number[]; max: number[] } | null = null;
  let home: { target: Vector3; position: Vector3 } | null = null;
  let zoomGoal = 0;
  const zoomDir = new Vector3();
  controls.addEventListener('start', () => (zoomGoal = 0));
  let userMoved = false;
  controls.addEventListener('start', () => (userMoved = true));
  const onResize = () => {
    resize();
    if (framed && !userMoved) frameRoom(framed.min, framed.max);
  };
  new ResizeObserver(onResize).observe(canvas);
  resize(); // now, not only on the observer's first callback: a fast hello must frame with the real aspect

  /**
   * Frames the room from the south-south-east, as close as fits the free area between the HUD panels.
   * Fits a sample of the drawn points (0.4% may fall outside, for stray scan fragments); before the points
   * arrive it fits the box from hello, which is loose because the scan's walls need not run along its axes.
   */
  function frameRoom(min: number[], max: number[]) {
    framed = { min, max };
    resize();
    const y0 = -max[2], y1 = Math.min(-min[2], CEILING_CUT_M);
    const pos = pointsGeom.getAttribute('position')?.array as Float32Array | undefined;
    const sample: number[] = [];
    if (pos && pointCount) {
      const stride = Math.ceil(pointCount / 3000);
      for (let i = 0; i < pointCount; i += stride) sample.push(pos[3 * i], pos[3 * i + 1], pos[3 * i + 2]);
    } else {
      for (let c = 0; c < 8; c++) sample.push(c & 1 ? max[1] : min[1], c & 2 ? y1 : y0, c & 4 ? -min[0] : -max[0]);
    }
    const count = sample.length / 3;
    let cx = 0, cz = 0;
    for (let i = 0; i < count; i++) {
      cx += sample[3 * i];
      cz += sample[3 * i + 2];
    }
    controls.target.set(cx / count, Math.min(0.8, y0 + (y1 - y0) * 0.3), cz / count);
    const polar = (44 * Math.PI) / 180, azim = (18 * Math.PI) / 180;
    tmp2.set(Math.sin(polar) * Math.sin(azim), Math.cos(polar), Math.sin(polar) * Math.cos(azim));
    const w = canvas.clientWidth, h = canvas.clientHeight;
    // NDC bounds of the free area: left of the Drone list, below the identity panel, above the dock.
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) / 15; // the HUD grows on large displays
    const right = 1 - (2 * (rightInset + 24)) / w, left = -1 + 48 / w, top = 1 - (300 * rem) / h, bottom = -1 + (390 * rem) / h;
    const allowedOutside = Math.floor(count * 0.004);
    const fits = () => {
      camera.updateMatrixWorld();
      let outside = 0;
      for (let i = 0; i < count; i++) {
        tmp.set(sample[3 * i], sample[3 * i + 1], sample[3 * i + 2]).project(camera);
        if ((tmp.x < left || tmp.x > right || tmp.y > top || tmp.y < bottom || tmp.z > 1) && ++outside > allowedOutside) return false;
      }
      return true;
    };
    let lo = 1, hi = 160;
    for (let i = 0; i < 24; i++) {
      const mid = (lo + hi) / 2;
      camera.position.copy(controls.target).addScaledVector(tmp2, mid);
      camera.lookAt(controls.target);
      if (fits()) hi = mid;
      else lo = mid;
    }
    camera.position.copy(controls.target).addScaledVector(tmp2, hi);
    pointsMat.uniforms.uHeight.value = Math.max(1, CEILING_CUT_M);
    pointsMat.uniforms.uFog.value.set(hi * 0.8, hi * 2.4);
    holoU.uFog.value.set(hi * 0.8, hi * 2.4);
    home = { target: controls.target.clone(), position: camera.position.clone() };
    controls.update();
  }

  // ---------- Input ----------

  let downX = 0, downY = 0, downAt = 0;
  let downDrone = 0;
  let downSpot: { n: number; e: number; d: number } | null = null;
  let hoverX = -1, hoverY = -1, hoverDirty = false;

  /** The Drone whose glyph circle or number chip is under (x, y), CSS px; the nearer of the two centres wins. */
  function droneAt(x: number, y: number): number {
    const w = canvas.clientWidth, h = canvas.clientHeight;
    // The chip's centre is 1 width right and 1.05 heights up from the glyph centre (label.center); the rect gets 6 px of slack.
    const cx = LABEL_PX.w * uiScale, cy = -1.05 * LABEL_PX.h * uiScale, hx = (LABEL_PX.w / 2) * uiScale + 6, hy = (LABEL_PX.h / 2) * uiScale + 6;
    let best = 0, bestD = Infinity;
    for (let i = 0; i < drones.length; i++) {
      const v = drones[i];
      if (!v.group.visible) continue;
      tmp.copy(v.pos).project(camera);
      if (tmp.z > 1) continue;
      const dx = ((tmp.x + 1) / 2) * w - x, dy = ((1 - tmp.y) / 2) * h - y;
      let d2 = dx * dx + dy * dy;
      if (d2 > DRONE_HIT_PX * DRONE_HIT_PX) {
        const px = -dx - cx, py = -dy - cy; // pointer relative to the chip centre
        d2 = Math.abs(px) <= hx && Math.abs(py) <= hy ? px * px + py * py : Infinity;
      }
      if (d2 < bestD) {
        bestD = d2;
        best = v.id;
      }
    }
    return best;
  }

  function spotAt(x: number, y: number) {
    if (!pointCount) return null;
    ndc.set((x / canvas.clientWidth) * 2 - 1, -(y / canvas.clientHeight) * 2 + 1);
    raycaster.setFromCamera(ndc, camera);
    // About 6 px of slack at the orbit distance.
    const px = (2 * camera.position.distanceTo(controls.target) * Math.tan((camera.fov * Math.PI) / 360)) / canvas.clientHeight;
    raycaster.params.Points.threshold = Math.min(0.2, Math.max(0.02, 6 * px));
    const hit = raycaster.intersectObject(cloud, false)[0];
    if (!hit) return null;
    const pos = pointsGeom.getAttribute('position').array as Float32Array; // the scan point itself, not the ray's closest point
    const i = hit.index!;
    return { n: -pos[3 * i + 2], e: pos[3 * i], d: -pos[3 * i + 1], at: hit.point };
  }

  function setHover(id: number) {
    if (id === hovered) return;
    const prev = byId.get(hovered);
    hovered = id;
    if (prev) applyLook(prev);
    const v = byId.get(id);
    if (v) applyLook(v);
    canvas.style.cursor = id ? 'pointer' : 'crosshair';
  }

  function showMarker(at: Vector3, state: typeof markerState) {
    marker.position.copy(at);
    marker.material.map = markerTex.pending;
    marker.material.opacity = state === 'provisional' ? 0.55 : 1;
    marker.visible = true;
    markerState = state;
  }

  canvas.addEventListener('pointermove', (e) => {
    hoverX = e.offsetX;
    hoverY = e.offsetY;
    hoverDirty = true; // resolved once per frame in the render loop
  });
  canvas.addEventListener('pointerleave', () => {
    hoverX = -1;
    hoverDirty = true;
  });
  canvas.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    downX = e.offsetX;
    downY = e.offsetY;
    downAt = performance.now();
    downDrone = droneAt(e.offsetX, e.offsetY);
    downSpot = null;
    if (downDrone) {
      pressed = downDrone;
      applyLook(byId.get(downDrone)!);
      return;
    }
    const s = spotAt(e.offsetX, e.offsetY);
    if (s) {
      downSpot = s;
      showMarker(s.at, 'provisional'); // feedback on press, before the click is confirmed
    }
  });
  const release = (e: PointerEvent, cancelled: boolean) => {
    const moved = Math.hypot(e.offsetX - downX, e.offsetY - downY) > CLICK_SLOP_PX;
    if (pressed) {
      const id = pressed;
      pressed = 0;
      applyLook(byId.get(id)!);
      if (!cancelled && !moved && droneAt(e.offsetX, e.offsetY) === id) on.drone(id, e.shiftKey);
    } else if (downSpot) {
      const s = downSpot;
      downSpot = null;
      if (cancelled || moved) {
        if (markerState === 'provisional') marker.visible = false; // it was an orbit drag, not a click
        return;
      }
      showMarker(s.at, 'pending');
      if (!on.spot({ n: s.n, e: s.e, d: s.d })) resolveMarker(false);
    }
  };
  canvas.addEventListener('pointerup', (e) => e.button === 0 && release(e, false));
  canvas.addEventListener('pointercancel', (e) => release(e, true));
  canvas.style.cursor = 'crosshair';

  function resolveMarker(approved: boolean) {
    if (!marker.visible || markerState !== 'pending') return;
    if (approved) {
      // The approved target stays marked as the active Command until the next one, Abort, Recall or reset.
      active.position.copy(marker.position);
      active.visible = true;
      marker.visible = false;
    } else {
      markerState = 'rejected';
      marker.material.map = markerTex.rejected;
      markerFadeAt = performance.now() + 700;
    }
  }

  // ---------- Render loop ----------

  let lastFrame = performance.now();
  renderer.setAnimationLoop(() => {
    const now = performance.now();
    const dt = Math.min(0.1, (now - lastFrame) / 1000);
    lastFrame = now;
    frames++;

    if (hoverDirty && !pressed) {
      hoverDirty = false;
      setHover(hoverX < 0 ? 0 : droneAt(hoverX, hoverY));
    }

    const rt = now - clockOffset - RENDER_DELAY_MS;
    let visible = 0;
    for (let i = 0; i < drones.length; i++) {
      const v = drones[i];
      if (!v.track.count || Number.isNaN(rt)) {
        v.group.visible = v.shadow.visible = v.label.visible = false;
        dropPos[i * 6 + 4] = dropPos[i * 6 + 1]; // zero length
        continue;
      }
      visible++;
      v.track.sample(rt, pose);
      v.pos.set(pose[1], -pose[2], -pose[0]);
      v.group.position.copy(v.pos);
      v.group.rotation.set(pose[4], -pose[5], -pose[3], 'YXZ'); // pitch, −yaw, −roll in Three.js axes
      v.label.position.copy(v.pos);
      v.group.visible = v.shadow.visible = v.label.visible = true;
      // Ground under the Drone: the Launch pad is on the floor, so both are at the floor's level.
      const onPad = pose[0] > padBox.n0 && pose[0] < padBox.n1 && pose[1] > padBox.e0 && pose[1] < padBox.e1 && pose[2] < padBox.d;
      const groundY = onPad ? -padBox.d + 0.004 : 0.004;
      v.shadow.position.set(v.pos.x, groundY, v.pos.z);
      const height = Math.max(0, v.pos.y - groundY);
      v.shadow.material.opacity = 0.75 * Math.max(0.25, 1 - height / 3);
      v.shadow.scale.setScalar(1 + height * 0.25);
      const o = i * 6;
      dropPos[o] = dropPos[o + 3] = v.pos.x;
      dropPos[o + 1] = v.pos.y - 0.03;
      dropPos[o + 4] = groundY;
      dropPos[o + 2] = dropPos[o + 5] = v.pos.z;
    }
    dropGeom.setDrawRange(0, drones.length * 2);
    dropGeom.attributes.position.needsUpdate = true;

    if (follow && visible) {
      followGoal.set(0, 0, 0);
      let n = 0;
      for (let i = 0; i < drones.length; i++) {
        const v = drones[i];
        if (!v.group.visible || (selected.size && !selected.has(v.id))) continue;
        followGoal.add(v.pos);
        n++;
      }
      if (n) {
        followGoal.divideScalar(n);
        const k = reducedMotion() ? 1 : 1 - Math.exp(-dt / 0.25);
        tmp2.subVectors(followGoal, controls.target).multiplyScalar(k);
        controls.target.add(tmp2);
        camera.position.add(tmp2);
      }
    }

    if (markerState === 'rejected' && marker.visible) {
      const f = (now - markerFadeAt) / 200;
      if (f >= 1) marker.visible = false;
      else if (f > 0) marker.material.opacity = 1 - f;
    }

    if (zoomGoal) {
      const dist = camera.position.distanceTo(controls.target);
      const next = dist + (zoomGoal - dist) * (reducedMotion() ? 1 : 1 - Math.exp(-dt * 12));
      zoomDir.subVectors(camera.position, controls.target).setLength(next);
      camera.position.copy(controls.target).add(zoomDir);
      if (Math.abs(next - zoomGoal) < zoomGoal * 0.002) zoomGoal = 0;
    }
    holoU.uTime.value = now / 1000;
    holoU.uMotion.value = reducedMotion() ? 0 : 1;
    controls.enableDamping = !reducedMotion();
    controls.dampingFactor = 1 - Math.pow(1 - 0.12, dt * 60); // OrbitControls damps per frame; this keeps the glide the same at 30 or 60 fps
    controls.update();
    renderer.render(scene, camera);
  });

  return {
    setRoom(h: Hello) {
      // Pad slot rings, and the pad's footprint for shadows.
      const seg = 32, ring = 0.13;
      const pos = new Float32Array(h.pad.length * seg * 6);
      let k = 0;
      for (const [n, e, d] of h.pad) {
        for (let s = 0; s < seg; s++) {
          for (const a of [s, s + 1]) {
            const t = (a / seg) * Math.PI * 2;
            pos[k++] = e + ring * Math.cos(t);
            pos[k++] = -d + 0.003;
            pos[k++] = -n + ring * Math.sin(t);
          }
        }
      }
      padLines.geometry.setAttribute('position', new BufferAttribute(pos, 3));
      const ns = h.pad.map((p) => p[0]), es = h.pad.map((p) => p[1]);
      padBox = { n0: Math.min(...ns) - 0.3, n1: Math.max(...ns) + 0.3, e0: Math.min(...es) - 0.3, e1: Math.max(...es) + 0.3, d: Math.max(...h.pad.map((p) => p[2])) };
      if (!helloSeen) frameRoom(h.room.min, h.room.max); // only the first hello of a page load: a reconnect keeps the operator's view
      helloSeen = true;
    },
    /** The scanned surface for the holographic layer: the /api/mesh body (counts, n,e,d f32 per vertex, u32 indices). */
    setMesh(buf: ArrayBuffer) {
      const head = new Uint32Array(buf, 0, 2);
      const ned = new Float32Array(buf, 8, head[0] * 3);
      const index = new Uint32Array(buf, 8 + head[0] * 12, head[1] * 3);
      const pos = new Float32Array(head[0] * 3);
      for (let i = 0; i < head[0]; i++) {
        pos[3 * i] = ned[3 * i + 1];
        pos[3 * i + 1] = -ned[3 * i + 2];
        pos[3 * i + 2] = -ned[3 * i];
      }
      const plan = buildPlan(pos, index, CEILING_CUT_M - 0.05);
      for (const o of [...holo.children]) {
        holo.remove(o);
        (o as Mesh).geometry.dispose();
      }
      const wallGeom = new BufferGeometry();
      wallGeom.setAttribute('position', new BufferAttribute(plan.walls, 3));
      wallGeom.setAttribute('aH', new BufferAttribute(plan.wallHeight, 1));
      const walls = new Mesh(wallGeom, wallMaterial(holoU));
      const edgeGeom = new BufferGeometry();
      edgeGeom.setAttribute('position', new BufferAttribute(plan.edges, 3));
      const edges = new LineSegments(edgeGeom, new LineBasicMaterial({ color: '#8eeaff', transparent: true, opacity: 0.9, blending: AdditiveBlending, depthWrite: false }));
      const f = plan.floor;
      const mask = new CanvasTexture(f.canvas);
      mask.flipY = false;
      const floorGeom = new PlaneGeometry(f.w, f.h).rotateX(-Math.PI / 2).translate(f.x0 + f.w / 2, 0.005, f.z0 + f.h / 2);
      const floorMesh = new Mesh(floorGeom, floorMaterial(holoU, mask, new Vector4(f.x0, f.z0, f.w, f.h)));
      for (const o of [floorMesh, walls, edges]) {
        o.frustumCulled = false;
        o.renderOrder = -1;
        holo.add(o);
      }
      holoU.uSweep.value.set(f.z0, f.z0 + f.h);
      holo.visible = holoOn;
      pointsMat.uniforms.uOpacity.value = holoOn ? 0.6 : 1;
      return { walls: plan.walls.length / 18 };
    },
    /** Zooms by a factor of the current distance (below 1 is closer), gliding there; clamped to the controls' range. */
    zoomBy(f: number) {
      const from = zoomGoal || camera.position.distanceTo(controls.target);
      zoomGoal = Math.min(controls.maxDistance, Math.max(controls.minDistance, from * f));
    },
    setHolo(on: boolean) {
      holoOn = on;
      holo.visible = on && holo.children.length > 0;
      pointsMat.uniforms.uOpacity.value = holo.visible ? 0.6 : 1;
    },
    /** Back to the framed overview of the whole scan. */
    resetView() {
      if (!home) return;
      zoomGoal = 0;
      controls.target.copy(home.target);
      camera.position.copy(home.position);
      controls.update();
    },
    /** Builds the cloud from (n, e, d) triples; the ceiling above CEILING_CUT_M is left out of the draw range. */
    setPoints(ned: Float32Array) {
      const count = ned.length / 3;
      const pos = new Float32Array(count * 3);
      let below = 0, above = count - 1;
      for (let i = 0; i < count; i++) {
        const y = -ned[3 * i + 2];
        const j = y <= CEILING_CUT_M ? below++ : above--; // below the cut first, so the draw range holds just those
        pos[3 * j] = ned[3 * i + 1];
        pos[3 * j + 1] = y;
        pos[3 * j + 2] = -ned[3 * i];
      }
      pointsGeom.dispose();
      pointsGeom.setAttribute('position', new BufferAttribute(pos, 3));
      pointsGeom.setDrawRange(0, below);
      pointsGeom.computeBoundingSphere();
      pointCount = below;
      if (framed && !userMoved) frameRoom(framed.min, framed.max);
      return { received: count, drawn: below };
    },
    pushState(s: State) {
      const arrival = performance.now();
      if (s.t_ms < lastT) {
        // The stream restarted (a new engine or mock): drop the old timeline.
        for (const v of drones) v.track.count = 0;
        clockOffset = NaN;
      }
      lastT = s.t_ms;
      const o = arrival - s.t_ms;
      // Track the least-delayed frame; drift upward slowly in case the server clock runs slow.
      clockOffset = Number.isNaN(clockOffset) || o < clockOffset ? o : clockOffset + (o - clockOffset) * 0.002;
      for (const d of s.drones) if (d.id > 0 && (byId.has(d.id) || drones.length < MAX_DRONES)) droneView(d.id).track.push(s.t_ms, d);
    },
    setSelected(ids: Iterable<number>) {
      selected.clear();
      for (const id of ids) selected.add(id);
      for (const v of drones) applyLook(v);
    },
    /** Dims the Drones and keeps them where they were last drawn, until a fresh state frame arrives. */
    setStale(on: boolean) {
      stale = on;
      for (const v of drones) applyLook(v);
    },
    setFollow(on: boolean) {
      follow = on;
      controls.enablePan = !on;
    },
    setRightInset(px: number) {
      rightInset = px;
      onResize();
    },
    resolveMarker,
    /** Clears the active target (Abort, Recall, reset). */
    clearActive() {
      active.visible = false;
      marker.visible = false;
    },
    /** Redraws Drone labels once the data font has loaded. */
    refreshLabels() {
      for (const v of drones) {
        for (const t of [v.tex.plain, v.tex.hover, v.tex.selected]) t.dispose();
        v.tex = { plain: labelTexture(v.id, false, false), hover: labelTexture(v.id, false, true), selected: labelTexture(v.id, true, false) };
        applyLook(v);
      }
    },
    /** Screen position (CSS px) of a Mission-frame point; test hook only. */
    project(n: number, e: number, d: number) {
      tmp.set(e, -d, -n).project(camera);
      return { x: ((tmp.x + 1) / 2) * canvas.clientWidth, y: ((1 - tmp.y) / 2) * canvas.clientHeight };
    },
    stats() {
      const gl = renderer.getContext();
      const info = gl.getExtension('WEBGL_debug_renderer_info');
      return {
        points: pointCount,
        drones: drones.filter((v) => v.group.visible).length,
        frames,
        renderer: String(gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER)),
        markerVisible: marker.visible,
      };
    },
  };
}
