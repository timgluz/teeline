import type { ParsedProblem } from 'teeline-wasm'

export function formatMetadata(problem: ParsedProblem): string {
  const cityCount = `${problem.cities.length} cities`
  const dt = problem.distanceType ? ` · ${problem.distanceType}` : ''
  return `parsed: ${cityCount}${dt}`
}

export function exampleUrl(name: string): string {
  return `/examples/${name}.tsp`
}

export function initUpload(parseFile: (input: string) => Promise<ParsedProblem>): void {
  // DOM elements — all pre-built in index.html
  const zoneTsp       = document.getElementById('zone-tsp')!
  const tspIdle       = document.getElementById('zone-tsp-idle')!
  const tspChip       = document.getElementById('zone-tsp-chip') as HTMLElement
  const tspChipName   = document.getElementById('tsp-chip-name')!
  const btnBrowse     = document.getElementById('btn-browse-tsp')!
  const inputTsp      = document.getElementById('input-tsp') as HTMLInputElement
  const btnReplaceTsp = document.getElementById('btn-replace-tsp')!

  const zoneOpt       = document.getElementById('zone-opt')!
  const optIdle       = document.getElementById('zone-opt-idle')!
  const optChip       = document.getElementById('zone-opt-chip') as HTMLElement
  const optChipName   = document.getElementById('opt-chip-name')!
  const btnReplaceOpt = document.getElementById('btn-replace-opt')!

  const metaLine  = document.getElementById('metadata-line') as HTMLElement
  const errorLine = document.getElementById('error-line') as HTMLElement

  // ---- TSP zone state transitions ----

  function showTspLoaded(filename: string, parsed: ParsedProblem): void {
    tspChipName.textContent = filename
    tspIdle.hidden = true
    tspChip.hidden = false
    zoneTsp.classList.add('drop-zone--loaded')
    metaLine.textContent = formatMetadata(parsed)
    metaLine.hidden = false
    errorLine.hidden = true
    advanceStepper()
  }

  function showTspIdle(): void {
    tspIdle.hidden = false
    tspChip.hidden = true
    tspChipName.textContent = ''
    zoneTsp.classList.remove('drop-zone--loaded')
    metaLine.hidden = true
    errorLine.hidden = true
    resetStepper()
  }

  function showError(msg: string): void {
    errorLine.textContent = `Error: ${msg}`
    errorLine.hidden = false
    metaLine.hidden = true
  }

  // ---- Parse a .tsp text string ----

  async function loadTspText(filename: string, text: string): Promise<void> {
    try {
      const parsed = await parseFile(text)
      showTspLoaded(filename, parsed)
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err))
    }
  }

  // ---- Drag-and-drop helper (reusable for both zones) ----

  function setupDragDrop(
    zone: HTMLElement,
    onDrop: (filename: string, text: string) => void,
  ): void {
    zone.addEventListener('dragover', (e) => {
      e.preventDefault()
      zone.classList.add('drop-zone--dragover')
    })
    zone.addEventListener('dragleave', () => zone.classList.remove('drop-zone--dragover'))
    zone.addEventListener('drop', (e) => {
      e.preventDefault()
      zone.classList.remove('drop-zone--dragover')
      const file = (e as DragEvent).dataTransfer?.files[0]
      if (!file) return
      const reader = new FileReader()
      reader.onload = () => onDrop(file.name, reader.result as string)
      reader.readAsText(file)
    })
  }

  // ---- Wire TSP zone ----

  setupDragDrop(zoneTsp, loadTspText)

  btnBrowse.addEventListener('click', () => inputTsp.click())
  inputTsp.addEventListener('change', (e) => {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => loadTspText(file.name, reader.result as string)
    reader.readAsText(file)
    inputTsp.value = '' // reset so the same file can be re-selected after replace
  })
  btnReplaceTsp.addEventListener('click', showTspIdle)

  // ---- Wire opt.tour zone (store only — no parse needed) ----

  setupDragDrop(zoneOpt, (filename) => {
    optChipName.textContent = filename
    optIdle.hidden = true
    optChip.hidden = false
    zoneOpt.classList.add('drop-zone--loaded')
  })
  btnReplaceOpt.addEventListener('click', () => {
    optIdle.hidden = false
    optChip.hidden = true
    optChipName.textContent = ''
    zoneOpt.classList.remove('drop-zone--loaded')
  })

  // ---- Wire example dataset buttons ----

  document.querySelectorAll<HTMLButtonElement>('[data-example]').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const name = btn.dataset.example!
      try {
        const resp = await fetch(exampleUrl(name))
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
        const text = await resp.text()
        await loadTspText(`${name}.tsp`, text)
      } catch (err) {
        showError(err instanceof Error ? err.message : String(err))
      }
    })
  })
}

// ---- Stepper helpers (module-private) ----

function advanceStepper(): void {
  const steps = document.querySelectorAll<HTMLElement>('.stepper-step')
  if (steps.length < 2) return
  steps[0].classList.add('stepper-step--done')
  steps[0].classList.remove('stepper-step--active')
  steps[0].removeAttribute('aria-current')
  steps[1].classList.add('stepper-step--active')
  steps[1].setAttribute('aria-current', 'step')
}

function resetStepper(): void {
  const steps = document.querySelectorAll<HTMLElement>('.stepper-step')
  if (steps.length < 2) return
  steps[0].classList.remove('stepper-step--done')
  steps[0].classList.add('stepper-step--active')
  steps[0].setAttribute('aria-current', 'step')
  steps[1].classList.remove('stepper-step--active')
  steps[1].removeAttribute('aria-current')
}
