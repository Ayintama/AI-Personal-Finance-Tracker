const API_BASE = "http://127.0.0.1:8080";
const SESSION_KEY = "ai-finance-session";
const UI_KEY = "ai-finance-ui";

const localClassifyRules = [
  { type: "expense", category: "餐饮", keywords: ["饭", "餐", "外卖", "奶茶", "咖啡", "火锅", "早餐", "午餐", "晚餐"] },
  { type: "expense", category: "交通", keywords: ["地铁", "公交", "打车", "滴滴", "高铁", "机票", "出租"] },
  { type: "expense", category: "购物", keywords: ["淘宝", "京东", "购物", "衣服", "数码", "超市"] },
  { type: "expense", category: "学习", keywords: ["书", "课程", "资料", "考试", "培训"] },
  { type: "expense", category: "娱乐", keywords: ["电影", "游戏", "会员", "演唱会", "旅游"] },
  { type: "expense", category: "医疗", keywords: ["药", "医院", "门诊", "体检"] },
  { type: "expense", category: "住房", keywords: ["房租", "水电", "物业", "宽带"] },
  { type: "income", category: "工资", keywords: ["工资", "薪资", "薪水"] },
  { type: "income", category: "奖金", keywords: ["奖金", "绩效", "年终"] },
  { type: "income", category: "兼职", keywords: ["兼职", "外包", "稿费"] },
  { type: "income", category: "报销", keywords: ["报销", "补贴"] },
  { type: "income", category: "理财", keywords: ["利息", "基金", "分红"] },
];

let state = {
  token: "",
  user: null,
  selectedMonth: getCurrentMonth(),
  records: [],
  categories: [],
  summary: emptySummary(),
  report: null,
};

const selectedRecordIds = new Set();

const els = {
  authPanel: document.querySelector("#authPanel"),
  authForm: document.querySelector("#authForm"),
  authUsername: document.querySelector("#authUsername"),
  authPassword: document.querySelector("#authPassword"),
  authEmail: document.querySelector("#authEmail"),
  authMessage: document.querySelector("#authMessage"),
  logoutBtn: document.querySelector("#logoutBtn"),
  currentUserName: document.querySelector("#currentUserName"),
  apiStatus: document.querySelector("#apiStatus"),
  navItems: document.querySelectorAll(".nav-item"),
  views: document.querySelectorAll(".view"),
  viewTitle: document.querySelector("#viewTitle"),
  monthInput: document.querySelector("#monthInput"),
  prevMonth: document.querySelector("#prevMonth"),
  nextMonth: document.querySelector("#nextMonth"),
  incomeMetric: document.querySelector("#incomeMetric"),
  expenseMetric: document.querySelector("#expenseMetric"),
  balanceMetric: document.querySelector("#balanceMetric"),
  budgetMetric: document.querySelector("#budgetMetric"),
  categoryBars: document.querySelector("#categoryBars"),
  recentRecords: document.querySelector("#recentRecords"),
  recordForm: document.querySelector("#recordForm"),
  typeInput: document.querySelector("#typeInput"),
  amountInput: document.querySelector("#amountInput"),
  categoryInput: document.querySelector("#categoryInput"),
  dateInput: document.querySelector("#dateInput"),
  remarkInput: document.querySelector("#remarkInput"),
  classifyPreview: document.querySelector("#classifyPreview"),
  filterType: document.querySelector("#filterType"),
  recordList: document.querySelector("#recordList"),
  selectAllRecords: document.querySelector("#selectAllRecords"),
  selectedCount: document.querySelector("#selectedCount"),
  bulkDeleteBtn: document.querySelector("#bulkDeleteBtn"),
  downloadTemplateBtn: document.querySelector("#downloadTemplateBtn"),
  exportExcelBtn: document.querySelector("#exportExcelBtn"),
  importFileInput: document.querySelector("#importFileInput"),
  importStatus: document.querySelector("#importStatus"),
  budgetForm: document.querySelector("#budgetForm"),
  budgetInput: document.querySelector("#budgetInput"),
  budgetRing: document.querySelector("#budgetRing"),
  budgetDetail: document.querySelector("#budgetDetail"),
  budgetAdvice: document.querySelector("#budgetAdvice"),
  budgetStatusText: document.querySelector("#budgetStatusText"),
  generateReportBtn: document.querySelector("#generateReportBtn"),
  aiReport: document.querySelector("#aiReport"),
};

function pad(value) {
  return String(value).padStart(2, "0");
}

function formatDate(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function formatMonth(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}`;
}

function getToday() {
  return formatDate(new Date());
}

function getCurrentMonth() {
  return formatMonth(new Date());
}

function emptySummary() {
  return {
    income: 0,
    expense: 0,
    balance: 0,
    budget: null,
    category_totals: {},
  };
}

function loadSession() {
  const saved = localStorage.getItem(SESSION_KEY);
  if (saved) {
    try {
      const session = JSON.parse(saved);
      state.token = session.token || "";
      state.user = session.user || null;
    } catch {
      localStorage.removeItem(SESSION_KEY);
    }
  }

  const ui = localStorage.getItem(UI_KEY);
  if (ui) {
    try {
      state.selectedMonth = JSON.parse(ui).selectedMonth || getCurrentMonth();
    } catch {
      state.selectedMonth = getCurrentMonth();
    }
  }
}

function saveSession() {
  localStorage.setItem(SESSION_KEY, JSON.stringify({ token: state.token, user: state.user }));
}

function saveUiState() {
  localStorage.setItem(UI_KEY, JSON.stringify({ selectedMonth: state.selectedMonth }));
}

function formatMoney(value) {
  return `￥${Number(value || 0).toFixed(2)}`;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function normalizeType(value) {
  const text = String(value || "").replace(/^\uFEFF/, "").trim().toLowerCase();
  if (["income", "收入", "收", "+"].includes(text)) return "income";
  if (["expense", "支出", "支", "-"].includes(text)) return "expense";
  return "";
}

function normalizeDate(value) {
  const text = String(value || "").replace(/^\uFEFF/, "").trim();
  const dateOnly = text.split(/\s+/)[0];
  if (/^\d{4}-\d{1,2}-\d{1,2}$/.test(text)) {
    const [year, month, day] = text.split("-");
    return `${year}-${pad(month)}-${pad(day)}`;
  }
  if (/^\d{4}-\d{1,2}-\d{1,2}$/.test(dateOnly)) {
    const [year, month, day] = dateOnly.split("-");
    return `${year}-${pad(month)}-${pad(day)}`;
  }
  if (/^\d{4}\/\d{1,2}\/\d{1,2}$/.test(text)) {
    const [year, month, day] = text.split("/");
    return `${year}-${pad(month)}-${pad(day)}`;
  }
  if (/^\d{4}\/\d{1,2}\/\d{1,2}$/.test(dateOnly)) {
    const [year, month, day] = dateOnly.split("/");
    return `${year}-${pad(month)}-${pad(day)}`;
  }
  return "";
}

function toDateTime(date) {
  return `${date} 12:00:00`;
}

function fromBackendRecord(item) {
  return {
    id: item.id,
    type: item.type,
    amount: Number(item.amount),
    categoryId: item.category_id,
    category: item.category_name,
    date: String(item.occurred_at || "").slice(0, 10),
    occurredAt: item.occurred_at,
    remark: item.remark || "",
  };
}

function monthRecords() {
  return state.records;
}

function getSummary() {
  const budget = state.summary.budget;
  const budgetAmount = budget ? Number(budget.amount) : 0;
  const expense = Number(state.summary.expense || 0);
  return {
    records: state.records,
    income: Number(state.summary.income || 0),
    expense,
    balance: Number(state.summary.balance || 0),
    budget: budgetAmount,
    budgetUsage: budgetAmount > 0 ? Math.round((expense / budgetAmount) * 100) : 0,
    categoryTotals: Object.fromEntries(
      Object.entries(state.summary.category_totals || {}).map(([name, amount]) => [name, Number(amount)]),
    ),
  };
}

async function apiRequest(path, options = {}) {
  const headers = {
    "Content-Type": "application/json",
    ...(options.headers || {}),
  };
  if (state.token) {
    headers.Authorization = `Bearer ${state.token}`;
  }

  let response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      ...options,
      headers,
    });
  } catch {
    throw new Error("连接后端失败，请确认 backend 已启动并监听 http://127.0.0.1:8080。");
  }

  const text = await response.text();
  let payload = null;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    throw new Error("后端返回格式异常，请检查服务日志。");
  }
  if (!response.ok || (payload && payload.code !== 0)) {
    const message = payload?.message || `请求失败：${response.status}`;
    if (response.status === 401 || payload?.code === 4002) {
      clearSession();
    }
    throw new Error(message);
  }
  return payload?.data;
}

async function login(username, password) {
  const data = await apiRequest("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password }),
  });
  state.token = data.token;
  state.user = data.user;
  saveSession();
}

async function register(username, password, email) {
  const body = { username, password };
  if (email) body.email = email;
  const data = await apiRequest("/api/auth/register", {
    method: "POST",
    body: JSON.stringify(body),
  });
  state.token = data.token;
  state.user = data.user;
  saveSession();
}

function clearSession() {
  state.token = "";
  state.user = null;
  state.records = [];
  state.categories = [];
  state.summary = emptySummary();
  state.report = null;
  selectedRecordIds.clear();
  localStorage.removeItem(SESSION_KEY);
  renderAll();
}

async function refreshData() {
  if (!state.token) {
    renderAll();
    return;
  }
  els.apiStatus.textContent = "正在同步后端数据";
  try {
    const [categories, transactions, summary] = await Promise.all([
      apiRequest("/api/categories"),
      apiRequest(`/api/transactions?month=${encodeURIComponent(state.selectedMonth)}&page_size=100`),
      apiRequest(`/api/statistics/monthly?month=${encodeURIComponent(state.selectedMonth)}`),
    ]);
    state.categories = categories || [];
    state.records = (transactions?.items || []).map(fromBackendRecord);
    selectedRecordIds.clear();
    state.summary = summary || emptySummary();
    state.report = null;
    renderAll();
    els.apiStatus.textContent = "已连接后端";
  } catch (error) {
    els.apiStatus.textContent = "后端同步失败";
    throw error;
  }
}

function categoriesByType(type) {
  return state.categories.filter((category) => category.type === type);
}

function localClassify(type, remark) {
  const text = String(remark || "").trim();
  const matched = localClassifyRules.find((rule) => rule.type === type && rule.keywords.some((keyword) => text.includes(keyword)));
  const fallbackName = type === "income" ? "其他收入" : "其他";
  const categoryName = matched?.category || fallbackName;
  const category = categoriesByType(type).find((item) => item.name === categoryName) || categoriesByType(type)[0];
  return {
    category,
    categoryName: category?.name || categoryName,
    source: matched ? "rule" : "fallback",
  };
}

async function classifyRecord(type, remark, amount = 0) {
  if (state.token && remark) {
    try {
      const data = await apiRequest("/api/ai/classify", {
        method: "POST",
        body: JSON.stringify({ remark, amount: String(amount || 0), type }),
      });
      const category = state.categories.find((item) => item.id === data.category_id);
      if (category) {
        return { category, categoryName: category.name, source: data.source || "api" };
      }
    } catch {
      // Keep the form usable if the classify endpoint is temporarily unavailable.
    }
  }
  return localClassify(type, remark);
}

async function ensureCategory(type, categoryName, remark, amount = 0) {
  const category = categoriesByType(type).find((item) => item.name === String(categoryName || "").trim());
  if (category) return category;
  const result = await classifyRecord(type, remark, amount);
  return result.category || categoriesByType(type)[0];
}

async function updateCategoryOptions() {
  if (!els.typeInput || !els.categoryInput || !els.classifyPreview) return;
  const type = els.typeInput.value;
  const options = categoriesByType(type);
  els.categoryInput.innerHTML = options.map((item) => `<option value="${item.id}">${item.name}</option>`).join("");

  if (!options.length) {
    els.classifyPreview.textContent = state.token ? "暂无分类，请检查后端分类数据" : "请先登录后加载分类";
    return;
  }

  const result = await classifyRecord(type, els.remarkInput.value, Number(els.amountInput.value || 0));
  if (result.category) {
    els.categoryInput.value = String(result.category.id);
  }
  els.classifyPreview.textContent = `推荐分类：${result.categoryName}（${result.source === "rule" ? "规则命中" : result.source === "api" ? "后端接口" : "默认分类"}）`;
}

function renderAuthState() {
  const loggedIn = Boolean(state.token && state.user);
  els.authPanel.classList.toggle("hidden", loggedIn);
  els.logoutBtn.classList.toggle("hidden", !loggedIn);
  els.currentUserName.textContent = loggedIn ? state.user.username : "未登录";
  els.apiStatus.textContent = loggedIn ? "已连接后端" : "请先登录";
}

function renderMetrics() {
  const summary = getSummary();
  els.incomeMetric.textContent = formatMoney(summary.income);
  els.expenseMetric.textContent = formatMoney(summary.expense);
  els.balanceMetric.textContent = formatMoney(summary.balance);
  els.budgetMetric.textContent = summary.budget ? `${summary.budgetUsage}%` : "未设置";
}

function renderCategoryBars() {
  const summary = getSummary();
  const entries = Object.entries(summary.categoryTotals).sort((a, b) => b[1] - a[1]);
  const max = Math.max(...entries.map(([, amount]) => amount), 1);

  if (!state.token) {
    els.categoryBars.innerHTML = `<div class="empty">登录后查看分类支出。</div>`;
    return;
  }
  if (entries.length === 0) {
    els.categoryBars.innerHTML = `<div class="empty">本月还没有支出记录。</div>`;
    return;
  }

  els.categoryBars.innerHTML = entries
    .map(([name, amount]) => {
      const width = Math.max(6, Math.round((amount / max) * 100));
      return `
        <div class="bar-row">
          <strong>${escapeHtml(name)}</strong>
          <div class="bar-track"><div class="bar-fill" style="width:${width}%"></div></div>
          <span>${formatMoney(amount)}</span>
        </div>
      `;
    })
    .join("");
}

function renderRecords() {
  const filter = els.filterType.value;
  const records = monthRecords()
    .filter((record) => filter === "all" || record.type === filter)
    .sort((a, b) => b.date.localeCompare(a.date));

  const visibleIds = new Set(records.map((record) => String(record.id)));
  [...selectedRecordIds].forEach((id) => {
    if (!visibleIds.has(id)) selectedRecordIds.delete(id);
  });

  els.recordList.innerHTML = records.length ? records.map((record) => renderRecordItem(record, true)).join("") : `<div class="empty">${state.token ? "当前筛选条件下暂无账单。" : "登录后查看账单明细。"}</div>`;
  els.recentRecords.innerHTML = records.slice(0, 5).length
    ? records.slice(0, 5).map((record) => renderRecordItem(record, false)).join("")
    : `<div class="empty">${state.token ? "本月还没有账单。" : "登录后查看最近账单。"}</div>`;
  updateBulkDeleteControls(records);

  document.querySelectorAll("[data-delete-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await apiRequest(`/api/transactions/${button.dataset.deleteId}`, { method: "DELETE" });
        await refreshData();
      } catch (error) {
        els.importStatus.textContent = error.message;
      }
    });
  });

  document.querySelectorAll("[data-select-id]").forEach((checkbox) => {
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) {
        selectedRecordIds.add(checkbox.dataset.selectId);
      } else {
        selectedRecordIds.delete(checkbox.dataset.selectId);
      }
      updateBulkDeleteControls(records);
    });
  });
}

function updateBulkDeleteControls(records = []) {
  if (!els.selectAllRecords || !els.selectedCount || !els.bulkDeleteBtn) return;
  const visibleIds = records.map((record) => String(record.id));
  const selectedVisibleCount = visibleIds.filter((id) => selectedRecordIds.has(id)).length;
  els.selectedCount.textContent = `已选 ${selectedVisibleCount} 条`;
  els.bulkDeleteBtn.disabled = selectedVisibleCount === 0;
  els.selectAllRecords.disabled = visibleIds.length === 0;
  els.selectAllRecords.checked = visibleIds.length > 0 && selectedVisibleCount === visibleIds.length;
  els.selectAllRecords.indeterminate = selectedVisibleCount > 0 && selectedVisibleCount < visibleIds.length;
}

function renderRecordItem(record, selectable = false) {
  const sign = record.type === "income" ? "+" : "-";
  const amountClass = record.type === "income" ? "amount-income" : "amount-expense";
  const recordId = String(record.id);
  return `
    <article class="record-item">
      ${selectable ? `
        <label class="record-check" title="选择账单">
          <input data-select-id="${escapeHtml(recordId)}" type="checkbox" ${selectedRecordIds.has(recordId) ? "checked" : ""} />
        </label>
      ` : ""}
      <div>
        <strong>${escapeHtml(record.category)} · ${escapeHtml(record.remark || "无备注")}</strong>
        <p>${escapeHtml(record.date)} · ${record.type === "income" ? "收入" : "支出"}</p>
      </div>
      <div>
        <strong class="${amountClass}">${sign}${formatMoney(record.amount)}</strong>
        <button data-delete-id="${record.id}" title="删除账单">删除</button>
      </div>
    </article>
  `;
}

function renderBudget() {
  const summary = getSummary();
  els.budgetInput.value = summary.budget || "";
  const usage = Math.min(summary.budgetUsage, 100);
  els.budgetRing.textContent = summary.budget ? `${summary.budgetUsage}%` : "0%";
  els.budgetRing.classList.toggle("over", summary.budgetUsage > 100);
  els.budgetRing.style.background = summary.budget
    ? `conic-gradient(${summary.budgetUsage > 100 ? "var(--danger)" : "var(--accent)"} ${usage * 3.6}deg, #e4ece6 0deg)`
    : "conic-gradient(var(--accent) 0deg, #e4ece6 0deg)";

  if (!state.token) {
    els.budgetStatusText.textContent = "未登录";
    els.budgetDetail.textContent = "登录后可同步后端预算。";
    els.budgetAdvice.textContent = "请先登录测试账号或注册新账号。";
    return;
  }
  if (!summary.budget) {
    els.budgetStatusText.textContent = "暂无预算";
    els.budgetDetail.textContent = "设置预算后可查看使用进度。";
    els.budgetAdvice.textContent = "建议先设置一个符合本月计划的总预算。";
    return;
  }

  const remaining = summary.budget - summary.expense;
  els.budgetStatusText.textContent = summary.budgetUsage > 100 ? "已超预算" : "预算正常";
  els.budgetDetail.textContent = `本月预算 ${formatMoney(summary.budget)}，已支出 ${formatMoney(summary.expense)}，剩余 ${formatMoney(remaining)}。`;
  els.budgetAdvice.textContent =
    summary.budgetUsage > 100
      ? "当前支出已经超过预算，建议暂停非必要消费。"
      : summary.budgetUsage >= 80
        ? "预算使用率较高，建议控制月底支出。"
        : "预算使用情况健康，可以继续保持。";
}

async function generateReport() {
  if (!state.token) {
    els.aiReport.innerHTML = `<p>请先登录后生成报告。</p>`;
    return;
  }
  try {
    const report = await apiRequest("/api/ai/report", {
      method: "POST",
      body: JSON.stringify({ month: state.selectedMonth }),
    });
    state.report = report;
    els.aiReport.innerHTML = `
      <h4>${escapeHtml(report.month)} 财务总结</h4>
      <p>本月收入 ${formatMoney(report.income)}，支出 ${formatMoney(report.expense)}，结余 ${formatMoney(report.balance)}。</p>
      <p>最高支出类别：${escapeHtml(report.top_category || "暂无")}。${report.generated_by_ai ? "由 AI 服务生成。" : "由本地规则生成。"}</p>
      <h4>优化建议</h4>
      <ul>${(report.suggestions || []).map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
    `;
  } catch (error) {
    els.aiReport.innerHTML = `<p>${escapeHtml(error.message)}</p>`;
  }
}

function buildWorkbookHtml(rows, title) {
  const bodyRows = rows
    .map(
      (record) => `
        <tr>
          <td>${escapeHtml(record.date)}</td>
          <td>${record.type === "income" ? "收入" : "支出"}</td>
          <td>${escapeHtml(record.category)}</td>
          <td>${Number(record.amount).toFixed(2)}</td>
          <td>${escapeHtml(record.remark)}</td>
        </tr>
      `,
    )
    .join("");

  return `
    <html xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">
      <head>
        <meta charset="UTF-8" />
        <style>
          table { border-collapse: collapse; }
          th, td { border: 1px solid #999; padding: 6px 10px; }
          th { background: #eaf3ef; font-weight: bold; }
        </style>
      </head>
      <body>
        <h3>${escapeHtml(title)}</h3>
        <table>
          <thead>
            <tr>
              <th>日期</th>
              <th>类型</th>
              <th>分类</th>
              <th>金额</th>
              <th>备注</th>
            </tr>
          </thead>
          <tbody>${bodyRows}</tbody>
        </table>
      </body>
    </html>
  `;
}

function downloadFile(content, filename, type) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function exportCurrentMonthExcel() {
  const rows = monthRecords().sort((a, b) => b.date.localeCompare(a.date));
  if (!rows.length) {
    els.importStatus.textContent = "本月暂无账单可导出。";
    return;
  }
  const html = buildWorkbookHtml(rows, `${state.selectedMonth} 账单明细`);
  downloadFile(html, `AI记账本-${state.selectedMonth}-账单.xls`, "application/vnd.ms-excel;charset=utf-8");
  els.importStatus.textContent = `已导出 ${rows.length} 条本月账单。`;
}

function downloadTemplate() {
  const rows = [
    { date: getToday(), type: "expense", category: "餐饮", amount: 25.5, remark: "午饭外卖" },
    { date: getToday(), type: "income", category: "工资", amount: 5200, remark: "本月工资" },
  ];
  const html = buildWorkbookHtml(rows, "AI 记账本导入模板");
  downloadFile(html, "AI记账本-导入模板.xls", "application/vnd.ms-excel;charset=utf-8");
  els.importStatus.textContent = "已下载导入模板，按表头填写后可重新上传。";
}

function splitCsvLine(line) {
  const cells = [];
  let current = "";
  let inQuotes = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    const next = line[index + 1];
    if (char === '"' && inQuotes && next === '"') {
      current += '"';
      index += 1;
    } else if (char === '"') {
      inQuotes = !inQuotes;
    } else if (char === "," && !inQuotes) {
      cells.push(current.trim());
      current = "";
    } else {
      current += char;
    }
  }
  cells.push(current.trim());
  return cells;
}

function parseCsv(text) {
  return text
    .split(/\r?\n/)
    .filter((line) => line.trim())
    .map(splitCsvLine);
}

function parseHtmlTable(text) {
  const doc = new DOMParser().parseFromString(text, "text/html");
  const rows = [...doc.querySelectorAll("table tr")];
  return rows.map((row) => [...row.children].map((cell) => cell.textContent.trim())).filter((row) => row.length);
}

async function parseImportedRows(matrix) {
  if (matrix.length < 2) return { rows: [], skipped: 0 };
  const headers = matrix[0].map((item) => String(item).replace(/^\uFEFF/, "").trim());
  const indexOf = (...names) => headers.findIndex((header) => names.includes(header));
  const dateIndex = indexOf("日期", "时间", "交易时间", "date", "Date", "time", "Time");
  const typeIndex = indexOf("类型", "type", "Type");
  const categoryIndex = indexOf("分类", "category", "Category");
  const amountIndex = indexOf("金额", "amount", "Amount");
  const remarkIndex = indexOf("备注", "remark", "Remark", "说明");

  if (dateIndex < 0 || typeIndex < 0 || amountIndex < 0) {
    if (dateIndex < 0 || amountIndex < 0) {
      throw new Error("表头至少需要包含：时间、金额；建议同时包含分类。");
    }
  }

  const rows = [];
  let skipped = 0;
  for (const row of matrix.slice(1)) {
    const rawAmount = Number(String(row[amountIndex] || "").replace(/[￥,\s]/g, ""));
    const rawCategory = categoryIndex >= 0 ? String(row[categoryIndex] || "").replace(/^\uFEFF/, "").trim() : "";
    const categoryType = state.categories.find((item) => item.name === rawCategory)?.type;
    const type = typeIndex >= 0
      ? normalizeType(row[typeIndex])
      : categoryType || (rawAmount < 0 ? "expense" : "expense");
    const date = normalizeDate(row[dateIndex]);
    const amount = Math.abs(rawAmount);
    const remark = remarkIndex >= 0 ? String(row[remarkIndex] || "").trim() : "";
    if (!type || !date || !Number.isFinite(amount) || amount <= 0) { skipped++; continue; }
    const category = await ensureCategory(type, rawCategory, remark, amount);
    if (!category) { skipped++; continue; }
    rows.push({ type, amount, categoryId: category.id, category: category.name, date, remark });
  }
  return { rows, skipped };
}

async function importTableFile(file) {
  if (!state.token) {
    els.importStatus.textContent = "请先登录后再导入账单。";
    return;
  }
  const extension = file.name.split(".").pop().toLowerCase();
  if (extension === "xlsx") {
    els.importStatus.textContent = "当前无依赖版本暂不支持 .xlsx，请使用模板 .xls 或 .csv。";
    return;
  }

  const text = await file.text();
  const matrix = extension === "csv" ? parseCsv(text) : parseHtmlTable(text);
  els.importStatus.textContent = "正在解析表格...";
  const { rows: imported, skipped } = await parseImportedRows(matrix);
  if (!imported.length) {
    const skipMsg = skipped > 0 ? `，跳过 ${skipped} 条无效行` : "";
    els.importStatus.textContent = `没有识别到有效账单，请检查时间、金额是否有值。${skipMsg}`;
    return;
  }

  const skipNote = skipped > 0 ? `（跳过 ${skipped} 条无效行）` : "";
  els.importStatus.textContent = `识别到 ${imported.length} 条账单${skipNote}，正在导入...`;
  let count = 0;
  for (const row of imported) {
    await apiRequest("/api/transactions", {
      method: "POST",
      body: JSON.stringify({
        category_id: row.categoryId,
        amount: String(row.amount),
        type: row.type,
        occurred_at: toDateTime(row.date),
        remark: row.remark,
      }),
    });
    count += 1;
    if (count % 25 === 0 || count === imported.length) {
      els.importStatus.textContent = `正在导入 ${count} / ${imported.length} 条账单...`;
    }
  }
  const doneMsg = skipped > 0 ? `已导入 ${count} 条账单到后端，跳过 ${skipped} 条无效行。` : `已导入 ${count} 条账单到后端。`;
  els.importStatus.textContent = doneMsg;
  await refreshData();
}

function renderAll() {
  els.monthInput.value = state.selectedMonth;
  renderAuthState();
  renderMetrics();
  renderCategoryBars();
  renderRecords();
  renderBudget();
  updateCategoryOptions();
}

function switchView(view) {
  const titles = {
    dashboard: "财务总览",
    records: "账单管理",
    budget: "预算管理",
    ai: "AI 财务报告",
  };
  els.viewTitle.textContent = titles[view];
  els.navItems.forEach((item) => item.classList.toggle("active", item.dataset.view === view));
  els.views.forEach((item) => item.classList.toggle("active", item.id === `${view}View`));
}

async function changeMonth(offset) {
  const [year, month] = state.selectedMonth.split("-").map(Number);
  const date = new Date(year, month - 1 + offset, 1);
  state.selectedMonth = formatMonth(date);
  saveUiState();
  await refreshData();
}

els.navItems.forEach((button) => button.addEventListener("click", () => switchView(button.dataset.view)));
els.prevMonth.addEventListener("click", () => changeMonth(-1));
els.nextMonth.addEventListener("click", () => changeMonth(1));
els.monthInput.addEventListener("change", async () => {
  state.selectedMonth = els.monthInput.value;
  saveUiState();
  await refreshData();
});
function showFieldError(inputEl, message) {
  clearFieldError(inputEl);
  if (!message) return;
  const err = document.createElement("span");
  err.className = "field-error";
  err.textContent = message;
  inputEl.classList.add("input-error");
  inputEl.parentNode.appendChild(err);
}

function clearFieldError(inputEl) {
  inputEl.classList.remove("input-error");
  const existing = inputEl.parentNode.querySelector(".field-error");
  if (existing) existing.remove();
}

els.typeInput.addEventListener("change", updateCategoryOptions);
els.remarkInput.addEventListener("input", updateCategoryOptions);
els.amountInput.addEventListener("input", () => {
  updateCategoryOptions();
  const val = parseFloat(els.amountInput.value);
  if (!els.amountInput.value) {
    clearFieldError(els.amountInput);
  } else if (!Number.isFinite(val) || val <= 0) {
    showFieldError(els.amountInput, "金额必须大于 0");
  } else if (val > 9999999.99) {
    showFieldError(els.amountInput, "金额不能超过 9,999,999.99");
  } else {
    clearFieldError(els.amountInput);
  }
});
els.dateInput.addEventListener("change", () => {
  if (!els.dateInput.value) {
    showFieldError(els.dateInput, "请选择日期");
  } else {
    clearFieldError(els.dateInput);
  }
});
els.filterType.addEventListener("change", renderRecords);
els.selectAllRecords.addEventListener("change", () => {
  const filter = els.filterType.value;
  const records = monthRecords()
    .filter((record) => filter === "all" || record.type === filter)
    .sort((a, b) => b.date.localeCompare(a.date));
  records.forEach((record) => {
    const id = String(record.id);
    if (els.selectAllRecords.checked) {
      selectedRecordIds.add(id);
    } else {
      selectedRecordIds.delete(id);
    }
  });
  renderRecords();
});
els.bulkDeleteBtn.addEventListener("click", async () => {
  const ids = [...selectedRecordIds];
  if (!ids.length) return;
  const confirmed = window.confirm(`确定删除选中的 ${ids.length} 条账单吗？删除后不能恢复。`);
  if (!confirmed) return;
  els.bulkDeleteBtn.disabled = true;
  els.selectedCount.textContent = `正在删除 ${ids.length} 条...`;
  try {
    let deleted = 0;
    for (const id of ids) {
      await apiRequest(`/api/transactions/${id}`, { method: "DELETE" });
      deleted += 1;
      els.selectedCount.textContent = `正在删除 ${deleted} / ${ids.length} 条...`;
    }
    selectedRecordIds.clear();
    await refreshData();
    els.importStatus.textContent = `已删除 ${ids.length} 条账单。`;
  } catch (error) {
    els.importStatus.textContent = error.message;
    await refreshData();
  }
});
els.generateReportBtn.addEventListener("click", generateReport);
els.exportExcelBtn.addEventListener("click", exportCurrentMonthExcel);
els.downloadTemplateBtn.addEventListener("click", downloadTemplate);
els.logoutBtn.addEventListener("click", clearSession);

els.authForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const action = event.submitter?.dataset.authAction || "login";
  els.authMessage.textContent = action === "login" ? "正在登录..." : "正在注册...";
  try {
    if (action === "login") {
      await login(els.authUsername.value.trim(), els.authPassword.value);
    } else {
      await register(els.authUsername.value.trim(), els.authPassword.value, els.authEmail.value.trim());
    }
    els.authMessage.textContent = "认证成功，正在加载后端数据...";
    await refreshData();
    els.authMessage.textContent = "登录成功，数据已同步。";
  } catch (error) {
    els.authMessage.textContent = error.message;
  }
});

els.importFileInput.addEventListener("change", async () => {
  const [file] = els.importFileInput.files;
  if (!file) return;
  try {
    await importTableFile(file);
  } catch (error) {
    els.importStatus.textContent = error.message;
  } finally {
    els.importFileInput.value = "";
  }
});

els.recordForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (!state.token) {
    els.importStatus.textContent = "请先登录后再保存账单。";
    return;
  }
  const amount = Number(els.amountInput.value);
  if (!Number.isFinite(amount) || amount <= 0) return;
  if (amount > 9999999.99) {
    showFieldError(els.amountInput, "金额不能超过 9,999,999.99");
    return;
  }
  if (!els.dateInput.value) {
    showFieldError(els.dateInput, "请选择日期");
    return;
  }
  const btn = els.recordForm.querySelector(".primary-btn");
  btn.disabled = true;
  btn.textContent = "保存中...";
  try {
    await apiRequest("/api/transactions", {
      method: "POST",
      body: JSON.stringify({
        category_id: Number(els.categoryInput.value),
        amount: String(amount),
        type: els.typeInput.value,
        occurred_at: toDateTime(els.dateInput.value),
        remark: els.remarkInput.value.trim(),
      }),
    });
    els.recordForm.reset();
    els.dateInput.value = getToday();
    els.amountInput.value = "";
    els.remarkInput.value = "";
    clearFieldError(els.amountInput);
    clearFieldError(els.dateInput);
    await refreshData();
  } catch (error) {
    els.importStatus.textContent = error.message;
  } finally {
    btn.disabled = false;
    btn.textContent = "保存账单";
  }
});

els.budgetForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (!state.token) return;
  const btn = els.budgetForm.querySelector(".primary-btn");
  btn.disabled = true;
  btn.textContent = "保存中...";
  try {
    await apiRequest("/api/budgets", {
      method: "POST",
      body: JSON.stringify({
        category_id: null,
        amount: String(Number(els.budgetInput.value || 0)),
        month: state.selectedMonth,
      }),
    });
    await refreshData();
  } catch (error) {
    els.budgetAdvice.textContent = error.message;
  } finally {
    btn.disabled = false;
    btn.textContent = "保存预算";
  }
});

async function init() {
  loadSession();
  els.dateInput.value = getToday();
  els.monthInput.value = state.selectedMonth;
  renderAll();
  if (state.token) {
    try {
      await refreshData();
    } catch (error) {
      els.authMessage.textContent = error.message;
      renderAll();
    }
  }
}

init();
