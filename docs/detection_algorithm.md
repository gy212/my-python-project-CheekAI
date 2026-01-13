# cheekAI 检测算法说明（Rust/Tauri 版本）

> **算法版本**: v2.0 (连续化评分)
> **更新日期**: 2024-12
> **主要改进**: 软阈值 + Logit 空间累加 + 置信度加权聚合

本文件用于记录当前 Rust/Tauri 实现的 AI 文本检测管线，方便后续维护与调参。主要代码位置：
- 入口命令：`src-tauri/src/api/detect.rs`
- 分段/预处理：`src-tauri/src/services/text_processor.rs`
- 本地评分：`src-tauri/src/services/detection/segment_builder.rs`
- LLM 评分（GLM/DeepSeek）：`src-tauri/src/services/detection/llm_analyzer.rs`
- 聚合/混合模式：`src-tauri/src/services/detection/{aggregation.rs,comparison.rs,dual_mode.rs}`

---

## 1. 入口与整体流程

### 1.1 单模式检测 `detect_text`
入口：`detect_text(request)`  
流程：
1. `normalize_punctuation` 统一标点/空白/换行。
2. `detect_language` 粗略判断语言（中文占比 >30% -> `zh`，否则 `en`）。
3. `build_paragraph_blocks` 生成段落块（见第 3 章）。
4. **若 `request.provider` 非空且配置了对应 key**：走 `build_segments_with_llm`（LLM 按段落逐段检测）。  
   否则走 `build_segments`（纯本地评分）。
5. `aggregate_segments` 聚合出整体概率与决策（见第 6 章）。
6. 若 `request.dual_mode=true`：额外跑一次纯本地 `dual_mode_detect` 作为对照（不影响主结果）。

### 1.2 混合/双模式检测 `detect_dual_mode`
入口：`detect_dual_mode(request)`  
流程：
1. 同样先 `normalize_punctuation` 和语言判断。
2. 生成段落块 `build_paragraph_blocks`。
3. 生成句子块 `build_sentence_blocks(text, 50, 200, 300)`（见第 3 章）。
4. 并行执行：
   - 段落侧：`build_paragraphs_batch_with_glm`（GLM 批量检测）。
   - 句子侧：`build_sentences_filtered_with_deepseek`（DeepSeek 过滤 + 并发检测）。
5. 分别 `aggregate_segments` 得到段落/句子整体概率。
6. `compare_dual_mode_results` 计算两侧一致性与差异段落（见第 7 章）。
7. `fuse_aggregations` 融合出混合模式整体概率（段落 0.6 + 句子 0.4）。

---

## 2. 文件预处理（TXT / DOCX / PDF）

入口：`preprocess_file(file_name, file_data)`  

### 2.1 TXT
- 优先 UTF‑8 解码，失败则用 lossy UTF‑8。

### 2.2 PDF
- 用 `pdf_extract::extract_text_from_mem` 提取文本。
- 去空行、逐行 trim。

### 2.3 DOCX（重点优化）
DOCX 主体位于 `word/document.xml`。处理步骤：
1. `extract_docx_paragraphs_from_document_xml`  
   - 轻量级 XML 扫描器，只收集 `<w:t>` 内文字。
   - **跳过以下标签及其子树文本**：  
     `w:tbl`（表格）、`w:drawing`/`w:pict`（图/图表/形状）、`w:object`（OLE 对象）、`w:txbxContent`（文本框）。  
   - 以 `</w:p>` 为段落边界，得到 raw paragraphs。
2. `is_noise_paragraph` 过滤明显噪声段（图表标题、数字表格等）：
   - 图/表标题：中文 “图X/表X…”，英文 “Figure/Fig./Table X …”。
   - 数字占比过高：非空字符中数字比例 > 0.6。
   - 极短且无句末标点且“数字/符号味重”的段落。
   - “符号汤”段落（字母/中文占比极低）。
3. 合并短段：
   - 过滤后按顺序拼接为 block，**每 block 至少 100 字符**；不足则并入下一/上一块。
4. 用 `\n\n` 连接 blocks，交给后续分段。

---

## 3. 分段策略

### 3.1 段落块 `build_paragraph_blocks`
输入为规范化后的纯文本：
1. 以空行（连续 `\n`）作为段落分隔。
2. 每个段落生成一个 `TextBlock`，记录：
   - `index / chunk_id`
   - `start / end`（**UTF‑8 字节偏移**，用于后续对齐）
   - `text`
3. **短标题块后处理** `postprocess_paragraph_blocks`：
   - 判定 `is_short_title_like`：非空字符 `<20` 且不包含句末标点（。！？.!?）。
   - 连续短标题块优先并入**后续正文块**（调整正文块 `start`）。
   - 若没有后续正文，则并入前一正文块；避免标题独立成段造成噪声。

### 3.2 句子块 `build_sentence_blocks`
用于混合模式句子侧：
1. `split_sentences_advanced` 按中英文句末标点切句，并记录 offset。  
   - 避免引号内切分、避免小数点误切。
2. 将句子按长度组合成块：
   - `target_chars=200`：尽量让块接近 200 字符。
   - `max_chars=300`：单句超过 300 字符则独立成块。
3. 输出 `TextBlock(label="sentence_block")`。

---

## 4. 本地特征计算（Stylometry）

函数：`compute_stylometry(text)`  
分词：正则 `"[A-Za-z0-9_]+|[\\u4e00-\\u9fff]"`（英文按词，中文按单字）。

输出指标：
- `ttr`（Type‑Token Ratio）：不同词数 / 总词数。  
  AI 往往更低（用词模板化）。
- `avg_sentence_len`：按 `split_sentences` 切句后的平均字符数。  
  过长/过短都会影响得分。
- `function_word_ratio`：功能词占比（小型中文功能词表）。
- `repeat_ratio`：词表中出现次数 >=3 的词占比（重复词越多越 AI）。
- `ngram_repeat_rate`：3‑gram 重复率（重复短语越多越 AI）。
- `punctuation_ratio`：标点占字符比。

---

## 5. 本地困惑度估计（Perplexity）

函数：`estimate_perplexity(text)`（启发式、对齐旧 Python 版）
1. 同样分词得到 tokens。
2. 统计 unigram 频率并计算熵：
   - `entropy = -Σ p(t) * ln(p(t))`
   - `ppl_uni = exp(entropy)`
3. 缩放与平滑：
   - `ppl_scaled = 20 + min(280, (ppl_uni - 1) * 22.5)`
   - `diversity = distinct_tokens / total_tokens`
   - `base_old = 120 - diversity*60 + len(chars)/500`
   - `ppl = 0.5*ppl_scaled + 0.5*base_old`
4. `ppl` clamp 到 `[20, 300]`，保留两位小数。

低 perplexity（更“可预测”）通常偏 AI。

---

## 6. 本地段落评分（连续化算法 v2）

函数：`make_segment -> score_segment_continuous`
输入：stylometry + `ngram_repeat_rate` + `ppl`（可选） + `text`（用于确定性扰动）。

### 6.1 算法设计理念

旧版算法使用"硬阈值 + 固定加减分"的阶梯函数，导致输出集中在少数几个值（如 0.50, 0.60, 0.78）。
新版算法采用：
- **Logit 空间累加**：在 log-odds 空间累加特征贡献，避免概率"顶死"到边界
- **软阈值（Sigmoid）**：用 sigmoid 函数替代硬阈值，实现平滑过渡
- **锚点改为强贡献项**：不再硬压分数到固定区间，而是作为额外的连续贡献
- **确定性扰动**：在临界区间应用基于文本 hash 的微小扰动，打破量化感

### 6.2 核心函数

```rust
// Sigmoid 软阈值：x 越小于 center，输出越接近 1
fn sigmoid(x: f64, center: f64, k: f64) -> f64 {
    1.0 / (1.0 + ((x - center) / k).exp())
}

// 反向 Sigmoid：x 越大于 center，输出越接近 1
fn sigmoid_inv(x: f64, center: f64, k: f64) -> f64 {
    1.0 - sigmoid(x, center, k)
}

// Logit 转概率
fn from_logit(logit: f64) -> f64 {
    1.0 / (1.0 + (-logit).exp())
}
```

### 6.3 特征贡献（Logit 空间）

起始 `logit = 0`（对应 p=0.5），各特征连续累加：

| 特征 | 贡献公式 | 最大贡献 | 说明 |
|------|----------|----------|------|
| TTR 低 | `sigmoid(ttr, 0.58, 0.08) * 1.2` | +1.2 | 低词汇多样性 → AI |
| TTR 高 | `sigmoid_inv(ttr, 0.78, 0.06) * (-0.9)` | -0.9 | 高词汇多样性 → 人类 |
| 重复率 | `sigmoid_inv(rep, 0.18, 0.06) * 1.0` | +1.0 | 高重复 → AI |
| N-gram | `sigmoid_inv(ngram, 0.10, 0.04) * 1.1` | +1.1 | 高短语重复 → AI |
| 句长短 | `sigmoid(avg_len, 35.0, 10.0) * 0.3` | +0.3 | 过短句子 |
| 句长长 | `sigmoid_inv(avg_len, 120.0, 25.0) * 0.4` | +0.4 | 过长句子 |
| PPL 低 | `sigmoid(ppl, 85.0, 20.0) * 1.0` | +1.0 | 低困惑度 → AI |
| PPL 高 | `sigmoid_inv(ppl, 200.0, 30.0) * (-0.6)` | -0.6 | 高困惑度 → 人类 |

### 6.4 锚点贡献（强信号）

锚点不再硬压分数，而是计算"强度"后作为额外贡献：

**AI 锚点**（低 ttr + 低 ppl + 高重复）：
```
strength = sigmoid(ttr, 0.55, 0.05)
         * sigmoid(ppl, 90.0, 15.0)
         * (sigmoid_inv(rep, 0.15, 0.04) + sigmoid_inv(ngram, 0.10, 0.03)) / 2
if strength > 0.3: logit += strength * 1.5
```

**Human 锚点**（高 ttr + 高 ppl + 低重复 + 合理句长）：
```
strength = sigmoid_inv(ttr, 0.70, 0.05)
         * sigmoid_inv(ppl, 170.0, 25.0)
         * sigmoid(rep, 0.15, 0.04)
         * sigmoid_inv(avg_len, 25.0, 8.0)
if strength > 0.3: logit += strength * (-1.2)
```

### 6.5 确定性扰动

在临界区间 `[0.35, 0.75]` 应用基于文本 hash 的微小扰动：
```rust
if prob > 0.35 && prob < 0.75 {
    let noise = deterministic_noise(text, 42) * 0.02;  // ±1% max
    prob = (prob + noise).clamp(0.02, 0.98);
}
```
- 同一文本、同一算法版本：结果永远一致
- 扰动幅度很小（±1%），不改变宏观判断
- 仅用于打破"量化感"，让分数更自然

### 6.6 置信度
`confidence = min(0.95, 0.55 + min(0.35, len(chars)/1800))`
文本越长置信度越高，上限 0.95。

---

## 7. LLM 检测路径

### 7.1 单模式 LLM（按段落逐段）
函数：`build_segments_with_llm`  
触发条件：`detect_text` 传入 `provider` 且对应 API key 存在。

行为：
1. 先用本地 `make_segment` 生成基底段落分数。
2. 对每个段落块 **串行** 调用 LLM：
   - provider=`deepseek:*` 用 `call_deepseek_json(model, ...)`
   - 其他 provider（如 GLM）用 `call_glm(model, ...)`
3. 单段落超时 60s；失败/超时则保留本地分数。

注意：当前是逐段串行，段落多 + `deepseek-reasoner` 时会很慢（可后续并发化）。

### 7.2 混合模式段落侧（GLM 批量）
函数：`build_paragraphs_batch_with_glm`
1. 先本地 `make_segment` 生成基底。
2. 将所有段落以 `[段落 chunk_id]` 形式拼 prompt，一次调用 `glm-4-flash`。
3. 超时 120s；失败则回退为本地结果。

### 7.3 混合模式句子侧（DeepSeek 过滤 + 并发）
函数：`build_sentences_filtered_with_deepseek`

**长度分流**
- `<10` 字符：直接丢弃（短句噪声）。
- `10–50` 字符：本地 stylometry 评分（不调 LLM）。
- `>=50` 字符：DeepSeek 检测  
  - `<300`：`deepseek-chat`  
  - `>=300`：`deepseek-reasoner`

**并发与重试**
- 并发上限：4（`DEEPSEEK_SENTENCE_MAX_CONCURRENCY=4`）。
- 每条最多 3 次尝试（`DEEPSEEK_SENTENCE_MAX_ATTEMPTS=3`），带 400ms/800ms 退避。
- 单次请求超时 60s（`DEEPSEEK_SENTENCE_TIMEOUT_SECS=60`）。
- 连续失败 -> 本地回退，并在 `explanations` 标记 `deepseek_retry_exhausted_local_fallback`。

**位置标记与对齐**
- DeepSeek prompt 会附带  
  `[chunk_id=<id> start=<start> end=<end>]`  
  便于回包后按 `chunk_id` 重新排序对齐。

---

## 8. 结果聚合与混合模式对比

### 8.1 单模式聚合 `aggregate_segments`（置信度加权 + 鲁棒统计）

**权重计算**（避免长段一票否决）：
```rust
weight = sqrt(len) * confidence.max(0.3)
```
- 使用 `sqrt(len)` 而非 `len`，减少长段落的主导作用
- 乘以置信度，让高置信度段落权重更大
- 置信度下限 0.3，避免低置信度段落被完全忽略

**鲁棒聚合**：
1. 加权平均：`weighted_prob = Σ(ai_probability * weight) / Σ(weight)`
2. Trimmed Mean（段落数 ≥5 时）：去掉最高/最低 10% 后取均值
3. 最终结果：`overall = 0.7 * weighted_prob + 0.3 * trimmed_prob`

**决策阈值**（`AggregationThresholds::default`）：
- `low=0.65`, `high=0.85`
- buffer margin=0.03
- `prob < low - margin` -> `pass`
- `low - margin <= prob < high - margin` -> `review`
- `>= high - margin` -> `flag`

### 8.2 混合模式对比 `compare_dual_mode_results`
- 计算段落/句子整体平均概率差 `probability_diff`。
- 对所有重叠段落对：
  - 若互相覆盖率都 >0.5，比较二者方向（>0.5 视为 AI）是否一致。
  - 方向一致计入 `consistency_score`。
  - 若概率差 `> diff_threshold(0.20)`，记录为 `divergent_region`，并截取 100 字预览。

### 8.3 融合结果
`fused_probability = 0.6*para + 0.4*sent`（若句子侧无结果则只用段落侧）。  
融合置信度同权重方式。

---

## 9. 性能与调参提示
- **检测耗时主要来自 LLM**：
  - 混合模式句子侧已 4 线并发；段落侧 GLM 批量一次请求。
  - 单模式 LLM 仍是逐段串行，段落多时建议改为并发或批量。
- **常用阈值位置**：
  - DOCX 跳过/噪声：`src-tauri/src/api/detect.rs`
  - 标题合并阈值（20 字）：`src-tauri/src/services/text_processor.rs`
  - 句子分流阈值（10/50/300）与并发（4）/重试（3）：`src-tauri/src/services/detection/llm_analyzer.rs`
  - 段落/句子融合权重（0.6/0.4）：`src-tauri/src/services/detection/dual_mode.rs`
- 若要“更激进地丢弃短句/短段”，可提升 `SENTENCE_MIN_LENGTH` 或调整 `is_short_title_like` 的长度判定。
