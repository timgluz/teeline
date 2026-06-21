import '@picocss/pico/css/pico.min.css'
import '../docs.css'
import { initTopbar } from '../topbar'
import { render, h } from 'preact'
import CSExplainer from './cs'

initTopbar()
const appEl = document.getElementById('app')
if (appEl) {
  render(h(CSExplainer, null), appEl)
}
