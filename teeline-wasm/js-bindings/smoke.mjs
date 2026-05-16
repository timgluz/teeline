import { solve } from './teeline_wasm.js';

const cities = [
  { id: 0, x: 565.0, y: 575.0 },
  { id: 1, x: 25.0,  y: 185.0 },
  { id: 2, x: 345.0, y: 750.0 },
  { id: 3, x: 945.0, y: 685.0 },
  { id: 4, x: 845.0, y: 655.0 },
];

const options = {
  epochs: 500, platooEpochs: 100,
  coolingRate: 0.0001, maxTemperature: 1000.0, minTemperature: 0.001,
  mutationProbability: 0.001, nElite: 3, nNearest: 3,
};

const result = solve('sa', cities, options);
console.assert(typeof result.total === 'number' && result.total > 0, 'positive tour distance');
console.assert(result.route.length === 5, 'visits all 5 cities');
const sorted = [...result.route].sort((a, b) => a - b);
console.assert(JSON.stringify(sorted) === '[0,1,2,3,4]', 'each city once');
console.log('JS smoke test PASSED — route:', result.route, 'distance:', result.total.toFixed(2));
