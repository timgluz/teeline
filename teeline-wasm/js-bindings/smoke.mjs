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
console.log('solve smoke PASSED — route:', result.route, 'distance:', result.total.toFixed(2));

// parseAndSolve — JSON input
import { parseAndSolve } from './teeline_wasm.js';
const jsonInput = JSON.stringify(cities);
const jsonResult = parseAndSolve('nn', jsonInput, options);
console.assert(typeof jsonResult.total === 'number' && jsonResult.total > 0, 'JSON: positive distance');
console.assert(jsonResult.route.length === 5, 'JSON: visits all 5 cities');
const jsonSorted = [...jsonResult.route].sort((a, b) => a - b);
console.assert(new Set(jsonSorted).size === 5, 'JSON: each city visited exactly once');
console.log('parseAndSolve(JSON) smoke PASSED — route:', jsonResult.route);

// parseAndSolve — TSPLIB input (note: TSPLIB IDs are 1-based)
const tsplibInput = `NAME: mini
NODE_COORD_SECTION
0 565.0 575.0
1 25.0 185.0
2 345.0 750.0
3 945.0 685.0
4 845.0 655.0
EOF
`;
const tsplibResult = parseAndSolve('nn', tsplibInput, options);
console.assert(typeof tsplibResult.total === 'number' && tsplibResult.total > 0, 'TSPLIB: positive distance');
console.assert(tsplibResult.route.length === 5, 'TSPLIB: visits all 5 cities');
console.assert(new Set([...tsplibResult.route]).size === 5, 'TSPLIB: each city visited exactly once');
console.log('parseAndSolve(TSPLIB) smoke PASSED — route:', tsplibResult.route);
