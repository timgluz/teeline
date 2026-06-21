import '@picocss/pico/css/pico.min.css'
import '../docs.css'
import { initTopbar } from '../topbar'
import { render, h } from 'preact'
import GSAExplainer from './gsa'

initTopbar()
const appEl = document.getElementById('app')
if (appEl) { render(h(GSAExplainer, null), appEl) }
