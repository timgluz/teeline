import '@picocss/pico/css/pico.min.css'
import '../docs.css'
import { initTopbar } from '../topbar'
import { render, h } from 'preact'
import FPAExplainer from './fpa'

initTopbar()
const appEl = document.getElementById('app')
if (appEl) {
  render(h(FPAExplainer, null), appEl)
}
