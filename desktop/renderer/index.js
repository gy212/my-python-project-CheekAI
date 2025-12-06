const inputText = document.getElementById('inputText')

const detectButton = document.getElementById('detectButton')

const sensitivitySelect = document.getElementById('sensitivity')

const providerSelect = document.getElementById('provider')

const summaryDiv = document.getElementById('summary')

const segmentsDiv = document.getElementById('segments')

const batchSummaryDiv = document.getElementById('batchSummary')

const batchItemsDiv = document.getElementById('batchItems')

const glmKeyInput = document.getElementById('glmKey')

const saveGlmButton = document.getElementById('saveGlm')

const fileInput = document.getElementById('fileInput')

const fileTriggerBtn = document.getElementById('fileTrigger')

const fileNameLabel = document.getElementById('fileName')

const uploadBtn = document.getElementById('uploadBtn')

const exportJsonBtn = document.getElementById('exportJson')

const exportCsvBtn = document.getElementById('exportCsv')

const toggleStructuredBtn = document.getElementById('toggleStructured')

const copyStructuredBtn = document.getElementById('copyStructured')

const structurePreviewDiv = document.getElementById('structurePreview')

const loadingMask = document.getElementById('loadingMask')
const loadingText = document.getElementById('loadingText')

if (typeof window !== 'undefined' && !window.dragEvent) {
  window.dragEvent = () => {}
}


if (fileTriggerBtn && fileInput) {

  fileTriggerBtn.addEventListener('click', () => fileInput.click())

}



const SENSITIVITY_PRESETS = {

  low: { chunkSizeTokens: 520, overlapTokens: 56 },

  medium: { chunkSizeTokens: 400, overlapTokens: 80 },

  high: { chunkSizeTokens: 260, overlapTokens: 108 },

}



function getChunkingForSensitivity(val) {

  const key = (val || 'medium').toLowerCase()

  const preset = SENSITIVITY_PRESETS[key] || SENSITIVITY_PRESETS.medium

  return { chunkSizeTokens: preset.chunkSizeTokens, overlapTokens: preset.overlapTokens }

}



async function fetchJSON(url, options = {}) {

  const rsp = await fetch(url, options)

  const text = await rsp.text()

  if (!rsp.ok) {

    const msg = text ? (() => { try { return JSON.parse(text).message || text } catch { return text } })() : `HTTP ${rsp.status}`

    throw new Error(typeof msg === 'string' ? msg : JSON.stringify(msg))

  }

  return text ? JSON.parse(text) : null

}

function setLoading(on, text = '正在检测...') {
  if (!loadingMask) return
  loadingMask.style.display = on ? 'flex' : 'none'
  if (loadingText) loadingText.textContent = text
  if (detectButton) detectButton.disabled = on
  if (uploadBtn) uploadBtn.disabled = on
  if (fileTriggerBtn) fileTriggerBtn.disabled = on
}

// 初始化时确保遮罩关闭，按钮可用
setLoading(false)



let detectInFlight = false
async function detect() {

  const text = inputText.value.trim()

  if (!text) return
  if (detectInFlight) return
  detectInFlight = true
  console.log('[detect] start', { length: text.length })
  setLoading(true, '正在调用 GLM 判别，请稍候...')

  const providers = []

  const p = providerSelect.value.trim()

  if (p) providers.push(p)

    const sensitivity = sensitivitySelect.value || 'medium'

  const chunking = getChunkingForSensitivity(sensitivity)

  const body = {

    text,

    providers,

    usePerplexity: true,

    useStylometry: true,

    preprocessOptions: { autoLanguage: true, stripHtml: true, redactPII: false, normalizePunctuation: true, chunkSizeTokens: chunking.chunkSizeTokens, overlapTokens: chunking.overlapTokens },

    chunking,

    sensitivity

  }

  lastDetectParams = body

  try {

    const data = await fetchJSON('http://127.0.0.1:8787/api/detect', {

      method: 'POST',

      headers: { 'Content-Type': 'application/json' },

      body: JSON.stringify(body)

    })

    lastBatch = null

    lastDetect = data
    console.log('[detect] success', { requestId: data.requestId, segments: data.segments?.length })

    renderResult(data)

  } catch (err) {

    console.error('detect_failed', err)

    alert(`检测失败：${err.message}`)

  } finally {
    detectInFlight = false
    setLoading(false)
  }

}



function renderResult(data) {

  if (!data || !Array.isArray(data.segments)) {

    return

  }

  const agg = data.aggregation

  if (agg) {

    const probPct = (agg.overallProbability * 100).toFixed(0)

    const confPct = (agg.overallConfidence * 100).toFixed(0)

    summaryDiv.innerHTML = `

      <div class="summary-row">

        <span class="pill">总体AI概率：${probPct}%</span>

        <span class="pill">置信度：${confPct}%</span>

      </div>

      <div style="display:flex;align-items:center;gap:12px;margin-top:8px">

        <div id="summaryProgressRing">

          <svg width="80" height="80" viewBox="0 0 80 80">

            <defs>

              <linearGradient id="progressGradient" x1="0%" y1="0%" x2="100%" y2="100%">

                <stop offset="0%" stop-color="#abd1c6"/>

                <stop offset="100%" stop-color="#f9bc60"/>

              </linearGradient>

            </defs>

            <circle class="progress-ring-circle progress-ring-bg" cx="40" cy="40" r="36"></circle>

            <circle class="progress-ring-circle progress-ring-fg" cx="40" cy="40" r="36"></circle>

          </svg>

          <div class="card-progress-text">${probPct}<span class="card-progress-sub">%</span></div>

        </div>

        <div class="legend">说明：AI概率越高越可疑；词汇多样性越高越像人类；困惑度越低越可能为AI。</div>

      </div>

    `

    renderActions()

  } else {

    summaryDiv.innerHTML = '<div class="summary-row"><span class="pill">预处理完成</span></div>'

  }



  segmentsDiv.innerHTML = ''

  for (const s of data.segments) {

    const card = document.createElement('div')

    card.className = 'card segment-card'

    const color = colorForProb(s.aiProbability)

    card.style.background = color

    card.dataset.chunkId = s.chunkId

    const ai = (s.aiProbability * 100).toFixed(0)

    const conf = (s.confidence * 100).toFixed(0)

    const stylometry = s.signals?.stylometry || {}

    const perplexity = s.signals?.perplexity || {}

    const ttr = (stylometry.ttr ?? 0).toFixed(2)

    const sent = (stylometry.avgSentenceLen ?? 0).toFixed(1)

    const pplValue = perplexity.ppl

    const ppl = typeof pplValue === 'number' ? pplValue.toFixed(2) : '-'

    card.innerHTML = `

      <div class="card-header segment-header">段 ${s.chunkId}</div>

      <div class="metrics">

        <div class="metric"><span class="metric-label" title="概率越高越可能为 AI">AI 概率</span><span class="metric-value">${ai}%</span></div>

        <div class="metric"><span class="metric-label" title="结果可信度">置信度</span><span class="metric-value">${conf}%</span></div>

        <div class="metric"><span class="metric-label" title="词汇多样性">词汇多样性</span><span class="metric-value">${ttr}</span></div>

        <div class="metric"><span class="metric-label" title="平均句子长度">平均句长</span><span class="metric-value">${sent}</span></div>

        <div class="metric"><span class="metric-label" title="困惑度，越低越可能为 AI">困惑度</span><span class="metric-value">${ppl}</span></div>

      </div>

    `

    card.classList.add('reveal')

    card.addEventListener('mouseenter', () => highlightNodesForChunk(s.chunkId, true))

    card.addEventListener('mouseleave', () => highlightNodesForChunk(s.chunkId, false))

    card.addEventListener('click', () => scrollToNodeForChunk(s.chunkId))

    segmentsDiv.appendChild(card)

  }

}



detectButton.addEventListener('click', detect)



async function refreshProviders() {
  try {
    const data = await fetchJSON('http://127.0.0.1:8787/api/providers')
    providerSelect.innerHTML = '<option value="">任意 LLM 判别</option>'
    for (const it of data.items || []) {
      for (const m of it.models || []) {
        const opt = document.createElement('option')
        opt.value = `${it.name}:${m}`
        opt.textContent = `${it.name}: ${m}`
        providerSelect.appendChild(opt)
      }
    }
    // Update custom select if it exists
    if (providerSelect.updateCustomOptions) {
      providerSelect.updateCustomOptions();
    }
  } catch {}
}



async function saveGlm() {

  const apiKey = glmKeyInput.value.trim()

  if (!apiKey) return

  if (window.secure && window.secure.setGlmKey) {

    const r = await window.secure.setGlmKey(apiKey)

    if (!r || !r.ok) {

      alert('保存失败：后端未就绪或网络错误，请稍后再试')

      return

    }

  }

  await refreshProviders()

}



saveGlmButton.addEventListener('click', saveGlm)

refreshProviders()



async function _loadSavedApiKey() {

  try {

    const rsp = await fetch('http://127.0.0.1:8787/api/config/file')

    const cfg = await rsp.json()

    const val = (((cfg || {}).data || {}).glm || {}).apiKey || ''

    if (typeof val === 'string' && val.trim()) {

      glmKeyInput.value = val.trim()

    }

  } catch {}

}



async function initApiKeyInput() {

  try {

    const checkRsp = await fetch('http://127.0.0.1:8787/api/config/glm/check')

    const check = await checkRsp.json()

    if (!check.present) {

      setTimeout(_loadSavedApiKey, 3000)

      return

    }

  } catch {}

  _loadSavedApiKey()

  setTimeout(_loadSavedApiKey, 3000)

}



initApiKeyInput()



function colorForProb(p) {

  // Interpolate between #00938a (Human) and #e16162 (AI)
  const r = Math.round(0 + (225 - 0) * p)
  const g = Math.round(147 + (97 - 147) * p)
  const b = Math.round(138 + (98 - 138) * p)

  return `rgba(${r}, ${g}, ${b}, 0.25)`

}



async function uploadPreprocess() {

  const f = fileInput.files[0]

  if (!f) {

    alert('璇峰厛閫夋嫨涓€涓枃浠跺悗鍐嶆墽琛岄澶勭悊')

    return

  }

  const sensitivity = sensitivitySelect.value || 'medium'

  const chunking = getChunkingForSensitivity(sensitivity)

  const form = new FormData()

  form.append('file', f)

  form.append('autoLanguage', 'true')

  form.append('stripHtml', 'true')

  form.append('redactPII', 'false')

  form.append('normalizePunctuationOpt', 'true')

  form.append('chunkSizeTokens', String(chunking.chunkSizeTokens))

  form.append('overlapTokens', String(chunking.overlapTokens))

  try {

    const rsp = await fetch('http://127.0.0.1:8787/api/preprocess/upload', { method: 'POST', body: form })

    if (!rsp.ok) {

      throw new Error(`HTTP ${rsp.status}`)

    }

    const data = await rsp.json()

    inputText.value = data.formattedText || data.normalizedText || ''

    lastStructured = data.structuredNodes || []

    lastFormattedText = data.formattedText || data.normalizedText || ''

    renderStructurePreview(data)

    try {
      console.log('[preprocess] auto trigger detect')
      await detect()
    } catch (err) {
      console.error('auto_detect_after_preprocess_failed', err)
    }

  } catch (err) {

    console.error('预处理失败：', err)

    alert(`预处理失败：${err.message}`)

  }

}



uploadBtn.addEventListener('click', uploadPreprocess)



async function batchDetect(files) {

  const providers = []

  const selectedProvider = providerSelect.value.trim()

  if (selectedProvider) providers.push(selectedProvider)

    const sensitivity = sensitivitySelect.value || 'medium'

  const chunking = getChunkingForSensitivity(sensitivity)

  const preprocessTasks = Array.from(files).map(async f => {

    const form = new FormData()

    form.append('file', f)

    form.append('autoLanguage', 'true')

    form.append('stripHtml', 'true')

    form.append('redactPII', 'false')

    form.append('normalizePunctuationOpt', 'true')

    form.append('chunkSizeTokens', String(chunking.chunkSizeTokens))

    form.append('overlapTokens', String(chunking.overlapTokens))

    const rsp = await fetch('http://127.0.0.1:8787/api/preprocess/upload', { method: 'POST', body: form })

    if (!rsp.ok) {

      throw new Error(`棰勫鐞嗘枃浠?${f.name} 澶辫触锛欻TTP ${rsp.status}`)

    }

    const pre = await rsp.json()

    return { file: f, pre }

  })

  try {

    const processed = await Promise.all(preprocessTasks)

    const items = processed.map(({ file, pre }) => ({

      id: file.name,

      text: pre.normalizedText,

      language: pre.preprocessSummary.language,

      providers,

      usePerplexity: true,

      useStylometry: true,

      preprocessOptions: { autoLanguage: true, stripHtml: true, redactPII: false, normalizePunctuation: true, chunkSizeTokens: chunking.chunkSizeTokens, overlapTokens: chunking.overlapTokens },

      chunking,

      sensitivity

    }))

    const body = { items, parallel: Math.min(4, items.length) || 1 }

    const data = await fetchJSON('http://127.0.0.1:8787/api/detect/batch', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })

    lastBatch = data

    renderBatch(data)

  } catch (err) {

    console.error('batch_detect_failed', err)

    alert(`鎵归噺妫€娴嬪け璐ワ細${err.message}`)

  }

}



function renderBatch(data) {

  const s = data.summary

  batchSummaryDiv.innerHTML = `鎵归噺缁撴灉锛氬叡 ${s.count} 鏉★紝澶辫触 ${s.failCount} 鏉★紝鍧囧€?${s.avgProbability.toFixed(2)}锛孭95 ${s.p95Probability.toFixed(2)}`

  batchItemsDiv.innerHTML = ''

  for (const it of data.items) {

    const card = document.createElement('div')

    card.className = 'card'

    const prob = it.aggregation.overallProbability

    card.style.background = colorForProb(prob)

    card.innerHTML = `${it.id} 姒傜巼 ${prob.toFixed(2)} 缃俊搴?${it.aggregation.overallConfidence.toFixed(2)}`

    batchItemsDiv.appendChild(card)

  }

}



fileInput.addEventListener('change', () => {

  const files = fileInput.files

  if (fileNameLabel) {

    if (files && files.length) {

      fileNameLabel.textContent = files.length === 1 ? files[0].name : `${files.length} 个文件已选择`

    } else {

      fileNameLabel.textContent = '未选择任何文件'

    }

  }

  if (files && files.length > 1) {

    batchDetect(files)

  }

})



function downloadBlob(filename, text) {

  const blob = new Blob([text], { type: 'text/plain' })

  const url = URL.createObjectURL(blob)

  const a = document.createElement('a')

  a.href = url

  a.download = filename

  document.body.appendChild(a)

  a.click()

  document.body.removeChild(a)

  URL.revokeObjectURL(url)

}



function toCsvFromDetect(data) {

  const lines = ['chunkId,probability,confidence,ttr,avgSentenceLen,ppl']

  for (const s of data.segments || []) {

    const row = [s.chunkId, s.aiProbability, s.confidence, s.signals.stylometry.ttr, s.signals.stylometry.avgSentenceLen, s.signals.perplexity.ppl ?? ''].join(',')

    lines.push(row)

  }

  return lines.join('\n')

}



function toCsvFromBatch(data) {

  const lines = ['id,probability,confidence']

  for (const it of data.items || []) {

    const row = [it.id, it.aggregation.overallProbability, it.aggregation.overallConfidence].join(',')

    lines.push(row)

  }

  return lines.join('\n')

}



function exportJson() {

  if (lastBatch) {

    downloadBlob('batch_result.json', JSON.stringify(lastBatch))

    return

  }

  if (lastDetect) {

    downloadBlob('detect_result.json', JSON.stringify(lastDetect))

  }

}



function exportCsv() {

  if (lastBatch) {

    downloadBlob('batch_result.csv', toCsvFromBatch(lastBatch))

    return

  }

  if (lastDetect) {

    downloadBlob('detect_result.csv', toCsvFromDetect(lastDetect))

  }

}



exportJsonBtn.addEventListener('click', exportJson)

exportCsvBtn.addEventListener('click', exportCsv)



let lastStructured = null

let lastFormattedText = null

let lastMapping = null



function renderStructurePreview(data) {

  const nodes = data.structuredNodes || []

  lastMapping = data.mapping || null

  if (!nodes.length) { structurePreviewDiv.style.display='none'; return }

  structurePreviewDiv.style.display='block'

  let html = '<div class="card-header">缁撴瀯棰勮</div>'

  for (let i = 0; i < nodes.length; i++) {

    const n = nodes[i]

    const badges = []

    if (lastMapping && lastMapping.nodeChunkMap && lastMapping.nodeChunkMap[i]) {

      const items = lastMapping.nodeChunkMap[i].slice(0,3)

      for (const it of items) { badges.push('<span class="pill">段 ' + it.chunkId + '</span>') }

    }

    if (n.type === 'heading') {
      html += '<div class="node" data-node-index="' + i + '" style="font-weight:600;margin-top:8px">## ' + n.text + ' ' + badges.join(' ') + '</div>'
    } else if (n.type === 'list_item') {
      html += '<div class="node" data-node-index="' + i + '">· ' + n.text + ' ' + badges.join(' ') + ' </div>'
    } else {
      html += '<div class="node" data-node-index="' + i + '" style="margin-top:6px">' + n.text + ' ' + badges.join(' ') + '</div>'
    }

  }

  structurePreviewDiv.innerHTML = html

}



toggleStructuredBtn.addEventListener('click', () => {

  structurePreviewDiv.style.display = (structurePreviewDiv.style.display==='none'?'block':'none')

})



copyStructuredBtn.addEventListener('click', () => {

  if (lastFormattedText) inputText.value = lastFormattedText

})



function scrollToNodeForChunk(chunkId) {

  // Ensure structure preview is visible
  if (structurePreviewDiv.style.display === 'none') {
    structurePreviewDiv.style.display = 'block';
  }

  if (!lastMapping || !lastMapping.segmentNodeMap) return

  const items = lastMapping.segmentNodeMap[chunkId] || []
  
  if (items.length === 0) return

  // Find the first node corresponding to this chunk
  const firstNodeIndex = items[0].nodeIndex

  const el = structurePreviewDiv.querySelector(`.node[data-node-index="${firstNodeIndex}"]`)

  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }

}



function highlightNodesForChunk(chunkId, on) {

  if (!lastMapping || !lastMapping.segmentNodeMap) return

  const items = lastMapping.segmentNodeMap[chunkId] || []

  for (const it of items) {

    const el = structurePreviewDiv.querySelector('.node[data-node-index="' + it.nodeIndex + '"]')

    if (el) { el.style.outline = on ? '2px solid #f9bc60' : 'none'; el.style.background = on ? 'rgba(249, 188, 96, 0.15)' : 'transparent' }

  }

}



structurePreviewDiv.addEventListener('mouseover', (e) => {

  const target = e.target.closest('.node')

  if (!target) return

  const idx = parseInt(target.dataset.nodeIndex)

  if (!lastMapping || !lastMapping.nodeChunkMap) return

  const chunks = (lastMapping.nodeChunkMap[idx] || []).map(x => x.chunkId)

  for (const card of segmentsDiv.querySelectorAll('.card')) {

    const cid = parseInt(card.dataset.chunkId || '0')

    if (chunks.includes(cid)) { card.style.outline = '2px solid #f9bc60' } else { card.style.outline = 'none' }

  }

})

structurePreviewDiv.addEventListener('mouseout', () => {

  for (const card of segmentsDiv.querySelectorAll('.card')) { card.style.outline = 'none' }

})



let lastDetect = null

let lastBatch = null

let lastDetectParams = null



function renderActions() {

  const actions = document.createElement('div')

  actions.style.display = 'flex'

  actions.style.gap = '8px'

  actions.style.marginTop = '12px'

  const keepBtn = document.createElement('button')

  keepBtn.textContent = '保留当前结果'

  const rerunBtn = document.createElement('button')

  rerunBtn.textContent = '重新检测'

  keepBtn.addEventListener('click', async () => {

    if (!lastDetect) return

    const reqParams = lastDetectParams || {}

    const body = {

      id: lastDetect.requestId || (Date.now()+''),

      reqParams,

      aggregation: lastDetect.aggregation,

      multiRound: lastDetect.multiRound || null,

    }

    try {

      await fetchJSON('http://127.0.0.1:8787/api/history/save', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })

      const data = await fetchJSON('http://127.0.0.1:8787/api/history/list')

      renderHistoryCompare(data)

    } catch (err) {

      console.error('history_save_failed', err)

      alert('保存历史失败：' + err.message)

    }

  })

  rerunBtn.addEventListener('click', async () => {

    await detect()

    try {

      const data = await fetchJSON('http://127.0.0.1:8787/api/history/list')

      renderHistoryCompare(data)

    } catch (err) {

      console.error('history_list_failed', err)

    }

  })

  summaryDiv.appendChild(actions)

}



function renderHistoryCompare(data) {

  const items = (data.items || []).slice(0, 2)

  if (!items.length) return

  const wrap = document.createElement('div')

  wrap.className = 'card'

  const rows = []

  for (const it of items) {

    const prob = (it.aggregation.overallProbability*100).toFixed(0)

    const conf = (it.aggregation.overallConfidence*100).toFixed(0)

    rows.push(it.ts + ' 概率 ' + prob + '% 置信度 ' + conf + '%')

  }

  wrap.innerHTML = '<div class="card-header">历史对比</div><div>' + rows.join('<br/>') + '</div>'

  summaryDiv.appendChild(wrap)

}

/* Custom Select Implementation */
function initCustomSelect(selectId) {
  const select = document.getElementById(selectId);
  if (!select) return;

  // Check if already initialized
  if (select.nextElementSibling && select.nextElementSibling.classList.contains('custom-select-container')) {
    return;
  }

  // Create container
  const container = document.createElement('div');
  container.className = 'custom-select-container';

  // Create trigger
  const trigger = document.createElement('div');
  trigger.className = 'custom-select-trigger';
  
  // Create options container
  const optionsDiv = document.createElement('div');
  optionsDiv.className = 'custom-select-options';

  container.appendChild(trigger);
  container.appendChild(optionsDiv);

  // Insert after select
  select.parentNode.insertBefore(container, select.nextSibling);
  select.classList.add('replaced');

  // Populate options
  function updateOptions() {
    optionsDiv.innerHTML = '';
    const selectedOption = select.options[select.selectedIndex];
    
    // Update trigger content
    let triggerText = selectedOption ? selectedOption.text : '';
    trigger.innerHTML = `
      <span class="custom-select-value">${triggerText}</span>
      <div class="arrow-icon">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
      </div>
    `;

    Array.from(select.options).forEach((opt, index) => {
      const div = document.createElement('div');
      div.className = 'custom-select-option';
      if (opt.selected) div.classList.add('selected');
      div.textContent = opt.text;
      div.dataset.value = opt.value;
      div.addEventListener('click', (e) => {
        e.stopPropagation();
        select.value = opt.value;
        select.dispatchEvent(new Event('change'));
        closeSelect();
      });
      optionsDiv.appendChild(div);
    });
  }

  updateOptions();

  // Event Listeners
  function toggleSelect(e) {
    e.stopPropagation();
    const isOpen = optionsDiv.classList.contains('open');
    
    // Close all other selects
    document.querySelectorAll('.custom-select-options.open').forEach(el => {
      el.classList.remove('open');
      el.previousElementSibling.classList.remove('open');
    });

    if (!isOpen) {
      optionsDiv.classList.add('open');
      trigger.classList.add('open');
    }
  }

  function closeSelect() {
    optionsDiv.classList.remove('open');
    trigger.classList.remove('open');
  }

  trigger.addEventListener('click', toggleSelect);

  // Sync when native select changes
  select.addEventListener('change', () => {
    updateOptions();
  });

  // Expose update function
  select.updateCustomOptions = updateOptions;
}

// Close all selects when clicking outside
document.addEventListener('click', () => {
  document.querySelectorAll('.custom-select-options.open').forEach(el => {
    el.classList.remove('open');
    el.previousElementSibling.classList.remove('open');
  });
});

// Initialize Custom Selects
initCustomSelect('sensitivity');
initCustomSelect('provider');

// Window Controls
const btnMin = document.getElementById('btn-min')
const btnRestore = document.getElementById('btn-restore')
const btnMax = document.getElementById('btn-max')
const btnClose = document.getElementById('btn-close')

if (btnMin) btnMin.addEventListener('click', () => window.secure && window.secure.minimize())
if (btnRestore) btnRestore.addEventListener('click', () => window.secure && window.secure.restore())
if (btnMax) btnMax.addEventListener('click', () => window.secure && window.secure.maximize())
if (btnClose) btnClose.addEventListener('click', () => window.secure && window.secure.close())




