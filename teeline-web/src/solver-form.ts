import type { AlgorithmInfo } from './wasm-types'
import { defaultSolveOptions, type SolveOptions } from './solver-options'

export function initSolverConfig(
  algorithms: AlgorithmInfo[],
  isProblemLoaded: () => boolean,
  onSolverReady: (solver: string, options: SolveOptions) => void,
): { refresh: () => void } {
  const step02       = document.getElementById('step-02') as HTMLElement
  const solverSelect = document.getElementById('solver-select') as HTMLSelectElement
  const configPanel  = document.getElementById('config-panel') as HTMLElement
  const btnRun       = document.getElementById('btn-run') as HTMLButtonElement
  const checkProblem = document.getElementById('check-problem') as HTMLElement
  const checkSolver  = document.getElementById('check-solver') as HTMLElement

  let selectedId: string | null = null
  let currentOptions: SolveOptions = defaultSolveOptions()

  // ---- Populate dropdown ----

  for (const s of algorithms) {
    const opt = document.createElement('option')
    opt.value = s.id
    opt.textContent = `${s.name} (${s.id})`
    solverSelect.appendChild(opt)
  }

  // ---- Solver selection ----

  function selectSolver(id: string): void {
    selectedId = id
    currentOptions = defaultSolveOptions()
    renderConfigPanel(id)
    highlightActivePill(id)
    solverSelect.value = id
    updateChecklist()
  }

  function highlightActivePill(id: string): void {
    document.querySelectorAll<HTMLButtonElement>('.pill[data-solver]').forEach((btn) => {
      btn.classList.toggle('pill--active', btn.dataset.solver === id)
      btn.ariaCurrent = btn.dataset.solver === id ? 'true' : 'false'
    })
  }

  // ---- Config panel rendering ----

  function renderConfigPanel(id: string): void {
    const solver = algorithms.find((a) => a.id === id)
    configPanel.innerHTML = ''

    if (!solver) return

    const kindBadge = document.createElement('p')
    kindBadge.className = 'solver-kind-badge'
    kindBadge.textContent = `${solver.kind} · ${solver.description}`
    configPanel.appendChild(kindBadge)

    if (solver.params.length === 0) {
      const hint = document.createElement('p')
      hint.className = 'muted config-no-params'
      hint.textContent = 'No configurable options for this solver.'
      configPanel.appendChild(hint)
      return
    }

    const grid = document.createElement('div')
    grid.className = 'config-grid'

    for (const param of solver.params) {
      const label = document.createElement('label')
      label.htmlFor = `param-${param.key}`
      label.textContent = param.label

      const input = document.createElement('input')
      input.type = 'number'
      input.id = `param-${param.key}`
      input.name = param.key
      input.value = String(currentOptions[param.key as keyof SolveOptions])
      if (param.min !== undefined) input.min = String(param.min)
      if (param.max !== undefined) input.max = String(param.max)
      if (param.step !== undefined) input.step = String(param.step)
      else if (param.valueType === 'int') input.step = '1'
      if (param.description) input.title = param.description

      input.addEventListener('input', () => {
        const raw = parseFloat(input.value)
        if (!isNaN(raw)) {
          ;(currentOptions as unknown as Record<string, number>)[param.key] = raw
        }
      })

      grid.appendChild(label)
      grid.appendChild(input)
    }

    configPanel.appendChild(grid)
  }

  // ---- Precondition checklist ----

  function updateChecklist(): void {
    const problemMet = isProblemLoaded()
    const solverMet  = selectedId !== null

    checkProblem.classList.toggle('checklist-item--met', problemMet)
    checkSolver.classList.toggle('checklist-item--met', solverMet)

    btnRun.disabled = !(problemMet && solverMet)
  }

  // ---- Wire pills ----

  document.querySelectorAll<HTMLButtonElement>('.pill[data-solver]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const id = btn.dataset.solver!
      selectSolver(id)
    })
  })

  // ---- Wire dropdown ----

  solverSelect.addEventListener('change', () => {
    if (solverSelect.value) selectSolver(solverSelect.value)
  })

  // ---- Run ----

  btnRun.addEventListener('click', () => {
    if (!selectedId || !isProblemLoaded()) return
    const step04 = document.getElementById('step-04') as HTMLElement
    step02.hidden = true
    step04.hidden = false
    advanceStepper(2)
    onSolverReady(selectedId, { ...currentOptions })
  })

  updateChecklist()
  return { refresh: updateChecklist }
}

// ---- Stepper helpers ----

function advanceStepper(toIndex: number): void {
  const steps = document.querySelectorAll<HTMLElement>('.stepper-step')
  steps.forEach((step, i) => {
    if (i < toIndex) {
      step.classList.add('stepper-step--done')
      step.classList.remove('stepper-step--active')
      step.removeAttribute('aria-current')
    } else if (i === toIndex) {
      step.classList.add('stepper-step--active')
      step.classList.remove('stepper-step--done')
      step.setAttribute('aria-current', 'step')
    } else {
      step.classList.remove('stepper-step--active', 'stepper-step--done')
      step.removeAttribute('aria-current')
    }
  })
}
