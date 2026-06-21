import '@picocss/pico/css/pico.min.css'
import '../docs.css'
import { initTopbar } from '../topbar'
import { render, h } from 'preact'
import GAExplainer from './ga'

initTopbar()
const appEl = document.getElementById('app')
if (appEl) {
  render(h(GAExplainer, null), appEl)
}
