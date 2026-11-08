import { enhanceCloud } from './enhance-cloud.js';

self.onmessage = ({ data: { id, points, ceiling } }) => {
  const start = performance.now();
  try {
    const result = enhanceCloud(points, { ceiling });
    if (!result.points.length) throw new Error('No connected scan samples');
    self.postMessage({ id, ...result, ms: performance.now() - start, enhanced: true }, [result.points.buffer]);
  } catch {
    self.postMessage({ id, points, received: points.length / 3, added: 0, enhanced: false }, [points.buffer]);
  }
};
