<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Archive,
  ArrowUpRight,
  Check,
  Clock3,
  Database,
  FolderOpen,
  History,
  LayoutDashboard,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Trash2,
  X,
  Pencil,
  Download,
  Terminal,
  HardDrive,
  Info,
} from '@lucide/vue';
import GitHubIcon from './components/GitHubIcon.vue';
import appIcon from './assets/gharchive.svg';
import packageInfo from '../package.json';
import coreInfo from '../ghboost-core.lock.json';
type Task = {
  id: number;
  name: string;
  repo_url: string;
  backup_dir: string;
  schedule_time: string;
  enabled: boolean;
  use_proxy: boolean;
  use_mirror: boolean;
  notes: string;
  last_status: string | null;
  last_error: string | null;
  next_run_at: string | null;
};
type Run = {
  id: number;
  task_id: number;
  started_at: string;
  finished_at: string | null;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  size_bytes: number;
  status: string;
  error: string | null;
  attempts: number;
};
type Config = {
  backup_root: string;
  concurrency: number;
  timeout_seconds: number;
  retries: number;
  retry_delay_seconds: number;
  minimum_free_bytes: number;
  mirror_url: string;
  require_hosts: boolean;
  paused: boolean;
  autostart: boolean;
  start_minimized: boolean;
  theme: string;
  accepted_notice: boolean;
  token_configured: boolean;
  auto_check_updates: boolean;
};
const desktop = isTauri();
const repository = 'https://github.com/jgbrzzh/GhArchive';
const pages = [
  { id: 'dashboard', name: '仪表盘', icon: LayoutDashboard },
  { id: 'tasks', name: '备份任务', icon: Archive },
  { id: 'history', name: '运行历史', icon: History },
  { id: 'settings', name: '设置', icon: Settings2 },
  { id: 'about', name: '关于', icon: Info },
];
const page = ref('dashboard');
const tasks = ref<Task[]>([]);
const history = ref<Run[]>([]);
const search = ref('');
const filter = ref('all');
const stats = ref({
  task_count: 0,
  enabled_count: 0,
  today_success: 0,
  size_bytes: 0,
  next_run_at: null as string | null,
  running: 0,
  paused: false,
});
const config = reactive<Config>({
  backup_root: '',
  concurrency: 2,
  timeout_seconds: 1800,
  retries: 3,
  retry_delay_seconds: 30,
  minimum_free_bytes: 104857600,
  mirror_url: '',
  require_hosts: false,
  paused: false,
  autostart: false,
  start_minimized: false,
  theme: 'dark',
  accepted_notice: false,
  token_configured: false,
  auto_check_updates: true,
});
const title = computed(() => pages.find((p) => p.id === page.value)?.name);
const visibleTasks = computed(() =>
  tasks.value.filter((t) =>
    (t.name + ' ' + t.repo_url).toLowerCase().includes(search.value.toLowerCase()),
  ),
);
const visibleRuns = computed(() =>
  history.value.filter((r) => filter.value === 'all' || r.status === filter.value),
);
const busy = ref(false);
const running = reactive(new Set<number>());
const error = ref('');
const toast = ref('');
type UpdateInfo = { checked: boolean; available?: boolean; version?: string; notes?: string };
const updateInfo = ref<UpdateInfo | null>(null);
const updateBusy = ref(false);
const updateInstalling = ref(false);
const updateStatus = ref('尚未检查更新');
let updateTimer: ReturnType<typeof setInterval>;
let unlistenUpdate: UnlistenFn | undefined;
const notice = ref(false);
const modal = ref(false);
const editing = ref<number | null>(null);
const log = ref<Run | null>(null);
const token = ref('');
const tokenEditing = ref(false);
const now = ref(Date.now());
const emptyTask = () => ({
  name: '',
  repo_url: '',
  backup_dir: config.backup_root,
  schedule_time: '03:00',
  enabled: true,
  use_proxy: true,
  use_mirror: false,
  notes: '',
});
const form = reactive(emptyTask());
let timer: ReturnType<typeof setInterval>;
let clock: ReturnType<typeof setInterval>;
const countdown = computed(() => {
  if (stats.value.paused) return '已暂停';
  if (!stats.value.next_run_at) return '尚无计划';
  const seconds = Math.max(0, Math.floor((Date.parse(stats.value.next_run_at) - now.value) / 1000));
  return seconds === 0
    ? '等待执行'
    : `${Math.floor(seconds / 3600)
        .toString()
        .padStart(2, '0')}:${Math.floor((seconds % 3600) / 60)
        .toString()
        .padStart(2, '0')}:${(seconds % 60).toString().padStart(2, '0')}`;
});
const label = (status: string | null) =>
  ({ success: '已完成', failed: '失败', running: '运行中', interrupted: '已中断' })[status || ''] ||
  '未运行';
const bytes = (n: number) =>
  n >= 1073741824
    ? `${(n / 1073741824).toFixed(2)} GB`
    : n >= 1048576
      ? `${(n / 1048576).toFixed(1)} MB`
      : `${(n / 1024).toFixed(1)} KB`;
const time = (s: string | null) =>
  s ? new Date(s).toLocaleString('zh-CN', { hour12: false }) : '—';
const duration = (r: Run) =>
  r.finished_at
    ? `${Math.max(0, Math.round((Date.parse(r.finished_at) - Date.parse(r.started_at)) / 1000))} 秒`
    : '执行中';
async function call<T>(name: string, payload: unknown = {}): Promise<T> {
  if (!desktop) throw new Error('请在 GhArchive 桌面应用中操作。浏览器仅显示界面预览。');
  const result = await invoke<{ success: boolean; data: T; error: { message: string } | null }>(
    'action',
    { name, payload },
  );
  if (!result.success) throw new Error(result.error?.message || '操作失败');
  return result.data;
}
function fail(e: unknown) {
  error.value = e instanceof Error ? e.message : String(e);
}
function message(s: string) {
  toast.value = s;
  setTimeout(() => (toast.value = ''), 4000);
}
async function refresh(loadConfig = false) {
  if (!desktop) return;
  try {
    const data = await Promise.all([
      call<typeof stats.value>('status'),
      call<Task[]>('list'),
      call<Run[]>('history', { limit: 200 }),
    ]);
    [stats.value, tasks.value, history.value] = data;
    if (loadConfig) {
      Object.assign(config, await call<Config>('config'));
      notice.value = !config.accepted_notice;
      applyTheme();
    }
  } catch (e) {
    fail(e);
  }
}
function applyTheme() {
  document.documentElement.dataset.theme =
    config.theme === 'system'
      ? matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
      : config.theme;
}
function add(t?: Task) {
  editing.value = t?.id ?? null;
  Object.assign(form, t || emptyTask());
  modal.value = true;
}
async function saveTask() {
  busy.value = true;
  try {
    await call('save', { id: editing.value, task: { ...form } });
    modal.value = false;
    message(editing.value ? '任务已更新' : '任务已添加');
    await refresh();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function remove(t: Task) {
  if (!confirm(`删除任务「${t.name}」及其运行历史？本地备份文件会保留。`)) return;
  try {
    await call('remove', { id: t.id, yes: true });
    await refresh();
    message('任务已删除，备份文件保留');
  } catch (e) {
    fail(e);
  }
}
async function toggle(t: Task) {
  try {
    await call('enable', { id: t.id, enabled: !t.enabled });
    await refresh();
  } catch (e) {
    fail(e);
  }
}
async function run(t?: Task) {
  if (t) running.add(t.id);
  else busy.value = true;
  try {
    await call(t ? 'run' : 'run-all', t ? { id: t.id } : {});
    message('备份完成');
  } catch (e) {
    fail(e);
  } finally {
    if (t) running.delete(t.id);
    else busy.value = false;
    await refresh();
  }
}
async function pause() {
  try {
    await call('set', { key: 'paused', value: !stats.value.paused });
    await refresh();
  } catch (e) {
    fail(e);
  }
}
async function accept() {
  try {
    await call('set', { key: 'accepted_notice', value: true });
    config.accepted_notice = true;
    void checkUpdates(false);
    notice.value = false;
  } catch (e) {
    fail(e);
  }
}
async function saveSettings() {
  busy.value = true;
  try {
    for (const key of [
      'backup_root',
      'concurrency',
      'timeout_seconds',
      'retries',
      'retry_delay_seconds',
      'minimum_free_bytes',
      'mirror_url',
      'require_hosts',
      'start_minimized',
      'theme',
      'autostart',
      'auto_check_updates',
    ] as const) {
      await call('set', { key, value: config[key] });
    }
    if (tokenEditing.value) {
      await call('set', { key: 'token', value: token.value });
      token.value = '';
      tokenEditing.value = false;
    }
    message('设置已保存');
    await refresh(true);
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function openGithub() {
  try {
    if (desktop) await call('github');
    else window.open(repository, '_blank', 'noopener,noreferrer');
  } catch (e) {
    fail(e);
  }
}
async function openRoot() {
  try {
    await call('open-root');
  } catch (e) {
    fail(e);
  }
}
function viewLog(t: Task) {
  log.value = history.value.find((r) => r.task_id === t.id) || null;
  if (!log.value) message('该任务暂无运行记录');
}
async function exportHistory() {
  try {
    const rows = await call<Run[]>('history', { limit: 1000 });
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(rows, null, 2)], { type: 'application/json' }),
    );
    const a = document.createElement('a');
    a.href = url;
    a.download = `gharchive-history-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    message('已导出最近 1000 条以内的记录');
  } catch (e) {
    fail(e);
  }
}
async function checkUpdates(force = true) {
  if (!desktop || updateBusy.value || !config.accepted_notice) return;
  updateBusy.value = true;
  if (force) updateStatus.value = '正在检查更新…';
  try {
    const result = await call<UpdateInfo>('update-check', { force });
    if (result.checked) {
      updateInfo.value = result;
      updateStatus.value = result.available ? `发现新版本 v${result.version}` : '当前已是最新版本';
      if (result.available && !force)
        message(`发现 GhArchive v${result.version}，可在设置中安装更新`);
    }
  } catch (e) {
    updateStatus.value = e instanceof Error ? e.message : String(e);
  } finally {
    updateBusy.value = false;
  }
}
async function installUpdate() {
  if (
    !window.confirm(
      '下载并验证更新后，GhArchive 将关闭并启动安装程序。备份任务运行时不能安装。是否继续？',
    )
  )
    return;
  updateBusy.value = true;
  updateInstalling.value = true;
  updateStatus.value = '正在准备更新…';
  try {
    const result = await call<UpdateInfo>('update-install');
    if (!result.available) {
      updateInfo.value = result;
      updateStatus.value = '当前已是最新版本';
    }
  } catch (e) {
    updateStatus.value = e instanceof Error ? e.message : String(e);
  } finally {
    updateBusy.value = false;
    updateInstalling.value = false;
  }
}
onMounted(async () => {
  if (desktop)
    unlistenUpdate = await listen<{ phase: string; downloaded?: number; total?: number }>(
      'update-progress',
      ({ payload }) => {
        updateStatus.value =
          payload.phase === 'download'
            ? `正在下载：${bytes(payload.downloaded || 0)}${payload.total ? ` / ${bytes(payload.total)}` : ''}`
            : payload.phase === 'verify'
              ? '正在验证更新签名…'
              : '正在启动安装程序…';
      },
    );
  await refresh(true);
  void checkUpdates(false);
  updateTimer = setInterval(() => void checkUpdates(false), 3600000);
  timer = setInterval(() => refresh(), 5000);
  clock = setInterval(() => (now.value = Date.now()), 1000);
});
onUnmounted(() => {
  clearInterval(updateTimer);
  unlistenUpdate?.();
  clearInterval(timer);
  clearInterval(clock);
});
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <img :src="appIcon" alt="" width="44" height="44" />
        <div>GhArchive<small>GITHUB BACKUP</small></div>
      </div>
      <div class="nav-label">工作空间</div>
      <nav>
        <button
          v-for="p in pages"
          :key="p.id"
          :class="{ selected: page === p.id }"
          @click="page = p.id"
        >
          <component :is="p.icon" :size="19" />{{ p.name
          }}<span v-if="p.id === 'tasks'" class="nav-count">{{ tasks.length }}</span>
        </button>
      </nav>
      <div class="sidebar-bottom">
        <div class="engine">
          <span class="dot" :class="{ muted: stats.paused || !desktop }"></span
          >{{ desktop ? (stats.paused ? '定时任务已暂停' : '本地备份引擎就绪') : '浏览器界面预览' }}
        </div>
        <small>GhBoost 访问 · 本地备份</small>
        <div class="sidebar-version">
          <button v-if="updateInfo?.available" class="text-button" @click="page = 'settings'">
            发现新版 v{{ updateInfo.version }}
          </button>
          GhArchive <span>v{{ packageInfo.version }}</span>
        </div>
      </div>
    </aside>
    <main>
      <header class="topbar">
        <div class="breadcrumb">
          工作空间 <span>/</span><strong>{{ title }}</strong>
        </div>
        <div class="header-actions">
          <span class="local-badge"><ShieldCheck :size="14" /> 数据保存在本地</span
          ><a
            :href="repository"
            class="icon-button github-link"
            title="在默认浏览器打开 GitHub 仓库"
            aria-label="在默认浏览器打开 GitHub 仓库"
            @click.prevent="openGithub"
            ><GitHubIcon
          /></a>
        </div>
      </header>
      <div class="content">
        <div v-if="!desktop" class="preview-banner">
          WebView 界面预览 · 实际备份、设置和日志请在桌面应用中使用。
        </div>
        <div v-if="error" class="error-banner" role="alert">
          {{ error }}<button aria-label="关闭错误" @click="error = ''"><X :size="16" /></button>
        </div>
        <div class="page-heading">
          <div>
            <div class="eyebrow">
              {{
                page === 'dashboard'
                  ? 'YOUR REPOSITORIES, PRESERVED.'
                  : 'LOCAL-FIRST REPOSITORY BACKUP'
              }}
            </div>
            <h1>{{ page === 'dashboard' ? '每一份代码，都有归处。' : title }}</h1>
            <p>
              {{
                page === 'dashboard'
                  ? '把值得保留的仓库交给计划任务，专注下一次创造。'
                  : page === 'tasks'
                    ? '按仓库安排每日备份，首次完整镜像，后续增量更新。'
                    : page === 'history'
                      ? '每次运行的结果、耗时和输出，都可以在这里追溯。'
                      : page === 'about'
                        ? '版本信息、项目主页与软件更新。'
                        : '为你的备份习惯，设置合适的运行方式。'
              }}
            </p>
          </div>
          <button
            v-if="page === 'dashboard'"
            class="primary"
            :disabled="!desktop || busy || !tasks.length"
            @click="run()"
          >
            <RefreshCw :size="17" :class="{ spin: busy }" />{{
              busy ? '备份中…' : '备份全部'
            }}</button
          ><button v-if="page === 'tasks'" class="primary" :disabled="!desktop" @click="add()">
            <Plus :size="18" />添加任务</button
          ><button v-if="page === 'history'" :disabled="!desktop" @click="exportHistory">
            <Download :size="17" />导出记录</button
          ><button
            v-if="page === 'settings'"
            class="primary"
            :disabled="busy || !desktop"
            @click="saveSettings"
          >
            <Check :size="17" />{{ busy ? '保存中…' : '保存设置' }}
          </button>
        </div>

        <template v-if="page === 'dashboard'">
          <div class="metrics">
            <article>
              <div class="metric-label">备份任务<Archive :size="18" /></div>
              <div class="metric-value">{{ stats.task_count }}<small>个</small></div>
              <p>
                <span class="green">{{ stats.enabled_count }} 个已启用</span
                ><span>每天自动执行</span>
              </p>
            </article>
            <article>
              <div class="metric-label">今日成功<Check :size="18" /></div>
              <div class="metric-value">{{ stats.today_success }}<small>次</small></div>
              <p>
                <span class="green">{{ stats.running ? '正在备份' : '等待下一次运行' }}</span>
              </p>
            </article>
            <article>
              <div class="metric-label">下次执行<Clock3 :size="18" /></div>
              <div class="metric-value countdown">{{ countdown }}</div>
              <p>
                <span>{{
                  stats.next_run_at ? time(stats.next_run_at) : '添加任务后开始计时'
                }}</span>
              </p>
            </article>
            <article>
              <div class="metric-label">备份占用<HardDrive :size="18" /></div>
              <div class="metric-value size">{{ bytes(stats.size_bytes) }}</div>
              <p><span>各任务最近成功备份的大小</span></p>
            </article>
          </div>
          <section class="overview-panel">
            <div>
              <div class="eyebrow">A LITTLE PEACE OF MIND</div>
              <h2>仓库在远方，备份在身边。</h2>
              <p>Git mirror 保存全部分支与标签。<br />按时同步，在你需要时随时取回。</p>
              <div class="overview-actions">
                <button
                  :disabled="!desktop"
                  @click="
                    page = 'tasks';
                    add();
                  "
                >
                  <Plus :size="16" />创建备份任务</button
                ><button class="text-button" :disabled="!desktop" @click="openRoot">
                  打开备份目录<ArrowUpRight :size="16" />
                </button>
              </div>
            </div>
            <div class="vault-art" aria-hidden="true">
              <div class="orbit o1"></div>
              <div class="orbit o2"></div>
              <div class="vault-center"><Archive :size="52" :stroke-width="1.4" /></div>
              <div class="art-chip chip-one"><Database :size="16" />本地镜像</div>
              <div class="art-chip chip-two"><ShieldCheck :size="16" />分支 · 标签</div>
            </div>
          </section>
          <section class="panel">
            <div class="panel-heading">
              <h2>最近的备份</h2>
              <button class="text-button" @click="page = 'history'">
                查看全部<ArrowUpRight :size="15" />
              </button>
            </div>
            <div v-if="!history.length" class="empty">
              <div class="empty-icon"><History :size="28" /></div>
              <h3>还没有备份记录</h3>
              <p>添加第一个仓库，运行一次备份。它会安静地出现在这里。</p>
              <button
                class="text-button"
                :disabled="!desktop"
                @click="
                  page = 'tasks';
                  add();
                "
              >
                添加一个仓库<Plus :size="16" />
              </button>
            </div>
            <div v-else class="recent-runs">
              <button
                v-for="r in history.slice(0, 5)"
                :key="r.id"
                class="recent-row"
                @click="log = r"
              >
                <div class="repo-avatar"><Archive :size="20" /></div>
                <div>
                  <strong>{{
                    tasks.find((t) => t.id === r.task_id)?.name || `任务 #${r.task_id}`
                  }}</strong
                  ><small>{{ time(r.started_at) }}</small>
                </div>
                <span class="badge" :class="r.status">{{ label(r.status) }}</span
                ><span class="muted-text">{{ duration(r) }}</span
                ><ArrowUpRight :size="16" />
              </button>
            </div>
          </section>
        </template>

        <template v-if="page === 'tasks'">
          <div class="toolbar">
            <label class="search"
              ><Search :size="17" /><input
                v-model="search"
                placeholder="搜索任务或仓库…"
                aria-label="搜索任务" /></label
            ><button class="text-button" :disabled="!desktop" @click="pause">
              <component :is="stats.paused ? Play : Pause" :size="16" />{{
                stats.paused ? '恢复定时' : '暂停定时'
              }}
            </button>
          </div>
          <section class="panel">
            <div v-if="!visibleTasks.length" class="empty tall">
              <div class="empty-icon"><Archive :size="30" /></div>
              <h3>{{ search ? '没有匹配的任务' : '从第一个仓库开始' }}</h3>
              <p>粘贴 GitHub 链接，选择目录和时间。其余交给 GhArchive。</p>
              <button v-if="!search" class="primary" :disabled="!desktop" @click="add()">
                <Plus :size="17" />添加备份任务
              </button>
            </div>
            <div v-else class="task-list">
              <article v-for="t in visibleTasks" :key="t.id" class="task-row">
                <div class="task-top">
                  <div class="repo-avatar"><Archive :size="20" /></div>
                  <div class="task-identity">
                    <h3>{{ t.name }}</h3>
                    <p>{{ t.repo_url }}</p>
                  </div>
                  <span class="badge" :class="t.last_status || 'idle'">{{
                    label(t.last_status)
                  }}</span
                  ><label class="switch"
                    ><input
                      :checked="t.enabled"
                      type="checkbox"
                      :aria-label="`启用 ${t.name}`"
                      :disabled="t.last_status === 'running'"
                      @change="toggle(t)" /><span></span
                  ></label>
                </div>
                <div class="task-details">
                  <span><Clock3 :size="14" />每天 {{ t.schedule_time }}</span
                  ><span><FolderOpen :size="14" />{{ t.backup_dir }}</span
                  ><span v-if="t.use_mirror" class="mirror-note">镜像</span
                  ><span v-if="t.use_proxy">内置 GhBoost 加速</span>
                </div>
                <p v-if="t.last_error" class="task-error">{{ t.last_error }}</p>
                <div class="task-bottom">
                  <small>下次执行 {{ time(t.next_run_at) }}</small>
                  <div>
                    <button class="text-button" @click="viewLog(t)">
                      <Terminal :size="15" />日志</button
                    ><button
                      class="icon-button"
                      title="编辑任务"
                      :disabled="t.last_status === 'running'"
                      @click="add(t)"
                    >
                      <Pencil :size="16" /></button
                    ><button
                      class="icon-button danger"
                      title="删除任务"
                      :disabled="t.last_status === 'running'"
                      @click="remove(t)"
                    >
                      <Trash2 :size="16" /></button
                    ><button
                      :disabled="running.has(t.id) || t.last_status === 'running'"
                      @click="run(t)"
                    >
                      <Play :size="14" />{{
                        running.has(t.id) || t.last_status === 'running' ? '运行中' : '立即备份'
                      }}
                    </button>
                  </div>
                </div>
              </article>
            </div>
          </section>
        </template>

        <template v-if="page === 'history'">
          <div class="toolbar">
            <div class="segmented">
              <button
                v-for="f in [
                  { id: 'all', name: '全部' },
                  { id: 'success', name: '成功' },
                  { id: 'failed', name: '失败' },
                  { id: 'running', name: '运行中' },
                  { id: 'interrupted', name: '中断' },
                ]"
                :key="f.id"
                :class="{ active: filter === f.id }"
                @click="filter = f.id"
              >
                {{ f.name }}
              </button>
            </div>
            <span class="muted-text">最近 {{ history.length }} 条记录</span>
          </div>
          <section class="panel">
            <div v-if="!visibleRuns.length" class="empty tall">
              <div class="empty-icon"><History :size="30" /></div>
              <h3>暂无运行记录</h3>
              <p>备份结束后，可以查看完整 Git 输出和错误信息。</p>
            </div>
            <table v-else>
              <thead>
                <tr>
                  <th>任务 / 开始时间</th>
                  <th>状态</th>
                  <th>耗时</th>
                  <th>大小</th>
                  <th>退出码</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in visibleRuns" :key="r.id">
                  <td>
                    <strong>{{
                      tasks.find((t) => t.id === r.task_id)?.name || `任务 #${r.task_id}`
                    }}</strong
                    ><small>{{ time(r.started_at) }}</small
                    ><span v-if="r.error" class="task-error">{{ r.error }}</span>
                  </td>
                  <td>
                    <span class="badge" :class="r.status">{{ label(r.status) }}</span>
                  </td>
                  <td>{{ duration(r) }}</td>
                  <td>{{ bytes(r.size_bytes) }}</td>
                  <td class="mono">{{ r.exit_code ?? '—' }}</td>
                  <td><button class="text-button" @click="log = r">查看日志</button></td>
                </tr>
              </tbody>
            </table>
          </section>
        </template>

        <template v-if="page === 'settings'">
          <section class="settings-section">
            <div>
              <h2>存储与运行</h2>
              <p>备份位置、并发和失败处理。</p>
            </div>
            <div class="panel settings-fields">
              <label
                >默认备份根目录<input v-model="config.backup_root" placeholder="D:\backups" /><small
                  >新任务使用此目录，现有任务的目录单独设置。</small
                ></label
              >
              <div class="field-grid">
                <label
                  >同时运行任务数<input
                    v-model.number="config.concurrency"
                    type="number"
                    min="1"
                    max="16" /></label
                ><label
                  >单个 Git 命令超时（秒）<input
                    v-model.number="config.timeout_seconds"
                    type="number"
                    min="1"
                    max="86400" /></label
                ><label
                  >失败重试次数<input
                    v-model.number="config.retries"
                    type="number"
                    min="0"
                    max="10" /></label
                ><label
                  >重试间隔（秒）<input
                    v-model.number="config.retry_delay_seconds"
                    type="number"
                    min="1"
                    max="3600"
                /></label>
              </div>
              <label
                >最低剩余空间（字节）<input
                  v-model.number="config.minimum_free_bytes"
                  type="number"
                  min="0"
                /><small>每次开始前检查；建议至少保留 100 MB，并按仓库大小调高。</small></label
              >
            </div>
          </section>
          <section class="settings-section">
            <div>
              <h2>GitHub 访问</h2>
              <p>自动调用内置 GhBoost 获取仓库，无需先打开 GhBoost。</p>
            </div>
            <div class="panel settings-fields">
              <label
                >公开仓库镜像地址<input
                  v-model="config.mirror_url"
                  placeholder="https://your-trusted-mirror.example"
                /><small>任务需单独启用镜像。配置 Token 后禁止镜像，避免凭据泄露。</small></label
              ><label
                >GitHub Token
                <span class="green">{{ config.token_configured ? '已加密保存' : '未配置' }}</span
                ><input
                  v-model="token"
                  type="password"
                  autocomplete="new-password"
                  placeholder="留空可清除已保存的 Token"
                  @input="tokenEditing = true"
                /><small>Windows DPAPI 按当前用户加密，只显示是否配置，无法回读明文。</small
                ><button
                  v-if="config.token_configured"
                  class="text-button danger"
                  @click="
                    token = '';
                    tokenEditing = true;
                  "
                >
                  清除已保存 Token
                </button></label
              ><label class="toggle-row"
                ><span>要求已应用 GhBoost Hosts<small>仅检查，不在后台修改系统 Hosts。</small></span
                ><span class="switch"
                  ><input v-model="config.require_hosts" type="checkbox" /><span></span></span
              ></label>
            </div>
          </section>
          <section class="settings-section">
            <div>
              <h2>桌面体验</h2>
              <p>托盘、启动和界面主题。</p>
            </div>
            <div class="panel settings-fields">
              <label class="toggle-row"
                ><span>开机自启<small>登录 Windows 后自动启动并恢复定时任务。</small></span
                ><span class="switch"
                  ><input v-model="config.autostart" type="checkbox" /><span></span></span></label
              ><label class="toggle-row"
                ><span
                  >启动最小化到托盘<small>关闭主窗口也会留在托盘，退出请使用托盘菜单。</small></span
                ><span class="switch"
                  ><input
                    v-model="config.start_minimized"
                    type="checkbox" /><span></span></span></label
              ><label
                >界面主题<select v-model="config.theme" @change="applyTheme">
                  <option value="dark">深色</option>
                  <option value="light">浅色</option>
                  <option value="system">跟随系统</option>
                </select></label
              >
            </div>
          </section>
          <section class="settings-section">
            <div>
              <h2>软件更新</h2>
              <p>当前版本 v{{ packageInfo.version }}</p>
            </div>
            <div class="panel settings-fields">
              <label class="toggle-row"
                ><span>自动检查更新<small>每天最多自动检查一次，发现新版后提示安装。</small></span
                ><span class="switch"
                  ><input v-model="config.auto_check_updates" type="checkbox" /><span></span></span
              ></label>
              <p role="status" aria-live="polite">{{ updateStatus }}</p>
              <p v-if="updateInfo?.notes" class="update-notes">{{ updateInfo.notes }}</p>
              <div class="update-actions">
                <button :disabled="!desktop || updateBusy" @click="checkUpdates(true)">
                  <RefreshCw :size="16" />{{
                    updateBusy && !updateInstalling ? '检查中…' : '检查更新'
                  }}
                </button>
                <button
                  v-if="updateInfo?.available"
                  :disabled="updateBusy || stats.running > 0"
                  @click="installUpdate"
                >
                  <Download :size="16" />{{ updateInstalling ? '更新中…' : '下载并安装' }}
                </button>
              </div>
              <small>更新包通过签名验证。安装时关闭应用，完成后重新启动。</small>
            </div>
          </section>
          <div class="settings-footer">
            <ShieldCheck :size="17" />所有任务与运行历史保存在本机。
          </div>
        </template>
        <template v-if="page === 'about'">
          <section class="settings-section">
            <div>
              <h2>GhArchive</h2>
              <p>GitHub 仓库定时备份器</p>
            </div>
            <div class="panel settings-fields about-fields">
              <p>软件版本：v{{ packageInfo.version }}</p>
              <p>GhBoost 核心：v{{ coreInfo.version }} · {{ coreInfo.revision.slice(0, 7) }}</p>
              <p>开源协议：GPL-3.0-only</p>
              <button @click="openGithub"><ArrowUpRight :size="16" />项目 GitHub 主页</button>
              <p role="status" aria-live="polite">{{ updateStatus }}</p>
              <p v-if="updateInfo?.notes" class="update-notes">{{ updateInfo.notes }}</p>
              <div class="update-actions">
                <button :disabled="!desktop || updateBusy" @click="checkUpdates(true)">
                  <RefreshCw :size="16" />{{
                    updateBusy && !updateInstalling ? '检查中…' : '检查更新'
                  }}
                </button>
                <button
                  v-if="updateInfo?.available"
                  :disabled="updateBusy || stats.running > 0"
                  @click="installUpdate"
                >
                  <Download :size="16" />{{ updateInstalling ? '更新中…' : '下载并安装' }}
                </button>
              </div>
              <small
                >自动检查可在设置中关闭。更新安装前会验证签名，且不会中断正在运行的备份。</small
              >
            </div>
          </section>
        </template>
      </div>
      <footer>
        <span><span class="dot"></span>LOCAL-FIRST · BUILT TO PRESERVE</span
        ><span>每天定时 · 本地备份</span>
      </footer>
    </main>
  </div>
  <div v-if="toast" class="toast" role="status"><Check :size="17" />{{ toast }}</div>
  <div v-if="notice" class="overlay">
    <section
      class="dialog notice-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="notice-title"
    >
      <div class="empty-icon"><ShieldCheck :size="28" /></div>
      <h2 id="notice-title">开始前，了解你的备份</h2>
      <p>GhArchive 将按你设置的时间访问 GitHub 并写入本地目录。请仅备份你有权访问的仓库。</p>
      <ul>
        <li>第三方镜像只用于公开仓库，配置 Token 时禁止镜像。</li>
        <li>Token 使用当前 Windows 用户的 DPAPI 加密。</li>
        <li>关闭窗口后定时任务仍在托盘中运行。</li>
        <li>关机和退出期间无法备份，重启后会补跑到期任务。</li>
        <li>删除任务会移除历史记录，但保留本地备份文件。</li>
      </ul>
      <button class="primary" @click="accept">我已了解，开始使用</button>
    </section>
  </div>
  <div v-if="modal" class="overlay" @click.self="modal = false">
    <form
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="task-title"
      @submit.prevent="saveTask"
    >
      <div class="dialog-heading">
        <h2 id="task-title">{{ editing ? '编辑备份任务' : '添加备份任务' }}</h2>
        <button type="button" class="icon-button" aria-label="关闭弹窗" @click="modal = false">
          <X :size="19" />
        </button>
      </div>
      <label
        >仓库链接<input
          v-model="form.repo_url"
          required
          placeholder="owner/repo 或 GitHub 链接"
          autofocus
        /><small>支持 HTTPS、git@github.com:owner/repo.git 和 owner/repo。</small></label
      ><label
        >任务名称<input v-model="form.name" placeholder="留空使用仓库名称" maxlength="200"
      /></label>
      <div class="field-grid">
        <label>每天执行时间<input v-model="form.schedule_time" type="time" required /></label
        ><label class="toggle-row"
          >启用任务<span class="switch"
            ><input v-model="form.enabled" type="checkbox" /><span></span></span
        ></label>
      </div>
      <label
        >备份根目录<input v-model="form.backup_dir" required placeholder="D:\backups" /><small
          >保存到 &lt;根目录&gt; / &lt;owner&gt; / &lt;repo&gt;.git</small
        ></label
      >
      <div class="checkboxes">
        <label><input v-model="form.use_proxy" type="checkbox" />使用内置 GhBoost 加速</label
        ><label><input v-model="form.use_mirror" type="checkbox" />使用公开仓库镜像</label>
      </div>
      <label
        >备注<textarea
          v-model="form.notes"
          rows="2"
          maxlength="10000"
          placeholder="可选，例如：每晚保留一份项目备份"
        />
      </label>
      <div class="dialog-footer">
        <button type="button" @click="modal = false">取消</button
        ><button type="submit" class="primary" :disabled="busy">
          {{ busy ? '保存中…' : '保存任务' }}
        </button>
      </div>
    </form>
  </div>
  <div v-if="log" class="overlay" @click.self="log = null">
    <section class="dialog log-dialog" role="dialog" aria-modal="true" aria-labelledby="log-title">
      <div class="dialog-heading">
        <h2 id="log-title">运行日志 #{{ log.id }}</h2>
        <button class="icon-button" aria-label="关闭日志" @click="log = null">
          <X :size="19" />
        </button>
      </div>
      <div class="log-meta">
        <span class="badge" :class="log.status">{{ label(log.status) }}</span
        >{{ time(log.started_at) }} · {{ duration(log) }} · {{ log.attempts }} 次尝试 · 退出码
        {{ log.exit_code ?? '—' }}
      </div>
      <p v-if="log.error" class="task-error">{{ log.error }}</p>
      <h3>stdout</h3>
      <pre>{{ log.stdout || '（无输出）' }}</pre>
      <h3>stderr</h3>
      <pre>{{ log.stderr || '（无输出）' }}</pre>
    </section>
  </div>
</template>
