import { SOLVERS, solverByAlias, paramsFor } from './solver-config'
import { defaultSolveOptions, type SolveOptions } from './solver-options'

export function initSolverConfig(
  isProblemLoaded: () => boolean,
  onSolverReady: (solver: string, options: SolveOptions) => void,
): void {
  const step02        = document.getElementById('step-02') as HTMLElement
  const step03        = document.getElementById('step-03') as HTMLElement
  const solverSelect  = document.getElementById('solver-select') as HTMLSelectElement
  const configPanel   = document.getElementById('config-panel') as HTMLElement
  const btnToSolve    = document.getElementById('btn-to-solve') as HTMLButtonElement
  const btnRun        = document.getElementById('btn-run') as HTMLButtonElement
  const checkProblem  = document.getElementById('check-problem') as HTMLElement
  const checkSolver   = document.getElementById('check-solver') as HTMLElement

  let selectedAlias: string | null = null
  let currentOptions: SolveOptions = defaultSolveOptions()

  // ---- Populate dropdown ----

  for (const s of SOLVERS) {
    const opt = document.createElement('option')
    opt.value = s.alias
    opt.textContent = `${s.name} (${s.alias})`
    solverSelect.appendChild(opt)
  }

  // ---- Solver selection ----

  function selectSolver(alias: string): void {
    selectedAlias = alias
    currentOptions = defaultSolveOptions()
    renderConfigPanel(alias)
    highlightActivePill(alias)
    solverSelect.value = alias
    updateChecklist()
  }

  function highlightActivePill(alias: string): void {
    document.querySelectorAll<HTMLButtonElement>('.pill[data-solver]').forEach((btn) => {
      btn.classList.toggle('pill--active', btn.dataset.solver === alias)
      btn.ariaCurrent = btn.dataset.solver === alias ? 'true' : 'false'
    })
  }

  // ---- Config panel rendering ----

  function renderConfigPanel(alias: string): void {
    const solver = solverByAlias(alias)
    const params = paramsFor(alias)

    configPanel.innerHTML = ''

    if (!solver) return

    const kind = document.createElement('p')
    kind.className = 'solver-kind-badge'
    kind.textContent = `${solver.kind} · ${solver.description}`
    configPanel.appendChild(kind)

    if (params.length === 0) {
      const hint = document.createElement('p')
      hint.className = 'muted config-no-params'
      hint.textContent = 'No configurable options for this solver.'
      configPanel.appendChild(hint)
      return
    }

    const grid = document.createElement('div')
    grid.className = 'config-grid'

    for (const param of params) {
      const label = document.createElement('label')
      label.htmlFor = `param-${param.key}`
      label.textContent = param.label

      const input = document.createElement('input')
      input.type = 'number'
      input.id = `param-${param.key}`
      input.name = param.key
      input.value = String(currentOptions[param.key])
      if (param.min !== undefined) input.min = String(param.min)
      if (param.max !== undefined) input.max = String(param.max)
      if (param.step !== undefined) input.step = String(param.step)
      else if (param.type === 'int') input.step = '1'
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
    const solverMet  = selectedAlias !== null

    checkProblem.classList.toggle('checklist-item--met', problemMet)
    checkSolver.classList.toggle('checklist-item--met', solverMet)

    btnRun.disabled = !(problemMet && solverMet)
  }

  // ---- Wire pills ----

  document.querySelectorAll<HTMLButtonElement>('.pill[data-solver]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const alias = btn.dataset.solver!
      selectSolver(alias)
    })
  })

  // ---- Wire dropdown ----

  solverSelect.addEventListener('change', () => {
    if (solverSelect.value) selectSolver(solverSelect.value)
  })

  // ---- Step 02 → Step 03 ----

  btnToSolve.addEventListener('click', () => {
    step02.hidden = true
    step03.hidden = false
    advanceStepper(2)
    updateChecklist()
  })

  // ---- Run ----

  btnRun.addEventListener('click', () => {
    if (!selectedAlias || !isProblemLoaded()) return
    const step04 = document.getElementById('step-04') as HTMLElement
    step03.hidden = true
    step04.hidden = false
    advanceStepper(3)
    onSolverReady(selectedAlias, { ...currentOptions })
  })
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
