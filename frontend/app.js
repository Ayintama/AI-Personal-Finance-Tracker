const STORAGE_KEY = "ai-finance-tracker-state";

const categories = {
  expense: ["餐饮", "交通", "购物", "学习", "娱乐", "医疗", "住房", "其他"],
  income: ["工资", "奖金", "兼职", "报销", "理财", "其他收入"],
};

const classifyRules = [
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

function createId() {
  if (window.crypto && typeof window.crypto.randomUUID === "function") {
    return window.crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function pad(value) {
  return String(value).padStart(2, "0");
}

function formatDate(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function formatMonth(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}`;
}

const initialRecords = [
  { id: createId(), type: "income", amount: 5200, category: "工资", date: getToday(), remark: "本月工资" },
  { id: createId(), type: "expense", amount: 32, category: "餐饮", date: getToday(), remark: "午饭外卖" },
  { id: createId(), type: "expense", amount: 8, category: "交通", date: getToday(), remark: "地铁通勤" },
  { id: createId(), type: "expense", amount: 168, category: "购物", date: getToday(), remark: "超市日用品" },
];

let state = loadState();

const els = {
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
  budgetForm: document.querySelector("#budgetForm"),
  budgetInput: document.querySelector("#budgetInput"),
  budgetRing: document.querySelector("#budgetRing"),
  budgetDetail: document.querySelector("#budgetDetail"),
  budgetAdvice: document.querySelector("#budgetAdvice"),
  budgetStatusText: document.querySelector("#budgetStatusText"),
  generateReportBtn: document.querySelector("#generateReportBtn"),
  aiReport: document.querySelector("#aiReport"),
};

function getToday() {
  return formatDate(new Date());
}

function getCurrentMonth() {
  return formatMonth(new Date());
}

function loadState() {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved) {
    return JSON.parse(saved);
  }
  return {
    selectedMonth: getCurrentMonth(),
    records: initialRecords,
    budgets: { [getCurrentMonth()]: 3500 },
  };
}

function saveState() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function formatMoney(value) {
  return `￥${Number(value || 0).toFixed(2)}`;
}

function monthRecords() {
  return state.records.filter((record) => record.date.startsWith(state.selectedMonth));
}

function getSummary() {
  const records = monthRecords();
  const income = records.filter((item) => item.type === "income").reduce((sum, item) => sum + item.amount, 0);
  const expense = records.filter((item) => item.type === "expense").reduce((sum, item) => sum + item.amount, 0);
  const budget = Number(state.budgets[state.selectedMonth] || 0);
  const categoryTotals = records
    .filter((item) => item.type === "expense")
    .reduce((acc, item) => {
      acc[item.category] = (acc[item.category] || 0) + item.amount;
      return acc;
    }, {});

  return {
    records,
    income,
    expense,
    balance: income - expense,
    budget,
    budgetUsage: budget > 0 ? Math.round((expense / budget) * 100) : 0,
    categoryTotals,
  };
}

function classifyRecord(type, remark) {
  const text = String(remark || "").trim();
  const matched = classifyRules.find((rule) => rule.type === type && rule.keywords.some((keyword) => text.includes(keyword)));
  return matched
    ? { category: matched.category, confidence: 0.9, source: "rule" }
    : { category: type === "income" ? "其他收入" : "其他", confidence: 0.4, source: "fallback" };
}

function updateCategoryOptions() {
  const type = els.typeInput.value;
  const result = classifyRecord(type, els.remarkInput.value);
  els.categoryInput.innerHTML = categories[type].map((name) => `<option value="${name}">${name}</option>`).join("");
  els.categoryInput.value = result.category;
  els.classifyPreview.textContent = `推荐分类：${result.category}（${result.source === "rule" ? "规则命中" : "默认分类"}）`;
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

  if (entries.length === 0) {
    els.categoryBars.innerHTML = `<div class="empty">本月还没有支出记录。</div>`;
    return;
  }

  els.categoryBars.innerHTML = entries
    .map(([name, amount]) => {
      const width = Math.max(6, Math.round((amount / max) * 100));
      return `
        <div class="bar-row">
          <strong>${name}</strong>
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

  els.recordList.innerHTML = records.length ? records.map(renderRecordItem).join("") : `<div class="empty">当前筛选条件下暂无账单。</div>`;
  els.recentRecords.innerHTML = records.slice(0, 5).length
    ? records.slice(0, 5).map(renderRecordItem).join("")
    : `<div class="empty">本月还没有账单。</div>`;

  document.querySelectorAll("[data-delete-id]").forEach((button) => {
    button.addEventListener("click", () => {
      state.records = state.records.filter((record) => record.id !== button.dataset.deleteId);
      saveState();
      renderAll();
    });
  });
}

function renderRecordItem(record) {
  const sign = record.type === "income" ? "+" : "-";
  const amountClass = record.type === "income" ? "amount-income" : "amount-expense";
  return `
    <article class="record-item">
      <div>
        <strong>${record.category} · ${record.remark || "无备注"}</strong>
        <p>${record.date} · ${record.type === "income" ? "收入" : "支出"}</p>
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

function generateReport() {
  const summary = getSummary();
  const topCategory = Object.entries(summary.categoryTotals).sort((a, b) => b[1] - a[1])[0];
  const suggestions = [];

  if (summary.expense === 0) {
    suggestions.push("本月暂无支出记录，可以先补充日常账单以获得更准确的分析。");
  }
  if (topCategory) {
    suggestions.push(`${topCategory[0]} 是本月最高支出类别，金额为 ${formatMoney(topCategory[1])}，建议检查是否存在可减少的非必要消费。`);
  }
  if (summary.budget && summary.budgetUsage >= 80) {
    suggestions.push(`预算使用率为 ${summary.budgetUsage}%，建议在月底前控制额外支出。`);
  }
  if (summary.balance > 0) {
    suggestions.push(`本月结余为 ${formatMoney(summary.balance)}，可以考虑将部分结余用于储蓄或学习投入。`);
  }
  if (suggestions.length < 2) {
    suggestions.push("建议保持每笔消费都添加备注，后续自动分类会更准确。");
  }

  els.aiReport.innerHTML = `
    <h4>${state.selectedMonth} 财务总结</h4>
    <p>本月收入 ${formatMoney(summary.income)}，支出 ${formatMoney(summary.expense)}，结余 ${formatMoney(summary.balance)}。</p>
    <p>${summary.budget ? `月度预算为 ${formatMoney(summary.budget)}，当前预算使用率 ${summary.budgetUsage}%。` : "本月尚未设置预算。"}</p>
    <h4>消费重点</h4>
    <p>${topCategory ? `支出最高的分类是 ${topCategory[0]}，占本月支出的 ${Math.round((topCategory[1] / summary.expense) * 100)}%。` : "暂无支出分类数据。"}</p>
    <h4>优化建议</h4>
    <ul>${suggestions.map((item) => `<li>${item}</li>`).join("")}</ul>
  `;
}

function renderAll() {
  els.monthInput.value = state.selectedMonth;
  renderMetrics();
  renderCategoryBars();
  renderRecords();
  renderBudget();
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

function changeMonth(offset) {
  const [year, month] = state.selectedMonth.split("-").map(Number);
  const date = new Date(year, month - 1 + offset, 1);
  state.selectedMonth = formatMonth(date);
  saveState();
  renderAll();
}

els.navItems.forEach((button) => button.addEventListener("click", () => switchView(button.dataset.view)));
els.prevMonth.addEventListener("click", () => changeMonth(-1));
els.nextMonth.addEventListener("click", () => changeMonth(1));
els.monthInput.addEventListener("change", () => {
  state.selectedMonth = els.monthInput.value;
  saveState();
  renderAll();
});
els.typeInput.addEventListener("change", updateCategoryOptions);
els.remarkInput.addEventListener("input", updateCategoryOptions);
els.filterType.addEventListener("change", renderRecords);
els.generateReportBtn.addEventListener("click", generateReport);

els.recordForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const amount = Number(els.amountInput.value);
  if (!Number.isFinite(amount) || amount <= 0) {
    return;
  }
  state.records.push({
    id: createId(),
    type: els.typeInput.value,
    amount,
    category: els.categoryInput.value,
    date: els.dateInput.value,
    remark: els.remarkInput.value.trim(),
  });
  els.recordForm.reset();
  els.dateInput.value = getToday();
  updateCategoryOptions();
  saveState();
  renderAll();
});

els.budgetForm.addEventListener("submit", (event) => {
  event.preventDefault();
  state.budgets[state.selectedMonth] = Number(els.budgetInput.value || 0);
  saveState();
  renderAll();
});

els.dateInput.value = getToday();
els.monthInput.value = state.selectedMonth;
updateCategoryOptions();
renderAll();
