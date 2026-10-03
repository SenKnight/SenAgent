/** 设置面板：左侧分节导航（通用 / 模型服务 / 技能 / 身份），三栏布局。
 *
 * 由现 `SettingsView.tsx` 重构：改为面板式（覆盖主区），文案接入 i18n，保留
 * 模型自动发现与「保存并生效」流程。
 */

import { useEffect, useState } from "react";

import { getRuntimeInfo, getSettings, listModels, saveSettings } from "../api";
import { useI18n } from "../hooks/useI18n";
import { useStore } from "../store";
import type {
  DraftSettings,
  EditableSettings,
  RuntimeInfo,
} from "../types";

/** 表单内 Provider 草稿：区分服务端回显与用户新输入。 */
interface ProviderDraft {
  name: string;
  base_url: string;
  model: string;
  wire_api: "chat" | "responses";
  /** 用户新输入；空串 = 不改动（保留服务端已存密钥）。env: 引用直接回显。 */
  apiKeyInput: string;
  /** 服务端已存密钥标记（明文不回显）。 */
  apiKeySet: boolean;
  max_tokens: number | null;
  temperature: number | null;
}

/** 左侧分节选择态。 */
type Sel =
  | { kind: "general" }
  | { kind: "provider"; index: number }
  | { kind: "skills" }
  | { kind: "identity" };

function toDrafts(s: EditableSettings): ProviderDraft[] {
  return s.providers.map((p) => ({
    name: p.name,
    base_url: p.base_url,
    model: p.model,
    wire_api: p.wire_api,
    apiKeyInput: p.api_key ?? "",
    apiKeySet: p.api_key_set,
    max_tokens: p.max_tokens,
    temperature: p.temperature,
  }));
}

const inputCls =
  "w-full bg-elevated border border-line rounded-md px-2 py-1.5 text-xs " +
  "text-ink placeholder:text-ink-faint focus:outline-none focus:border-line-strong";

export function SettingsPanel() {
  const { t } = useI18n();
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);
  const closeSettings = useStore((s) => s.closeSettings);

  const [loaded, setLoaded] = useState<EditableSettings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<ProviderDraft[]>([]);
  const [defaultProvider, setDefaultProvider] = useState("");
  const [contextWindow, setContextWindow] = useState(128000);
  const [maxToolRounds, setMaxToolRounds] = useState(25);
  const [systemPrompt, setSystemPrompt] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<{
    kind: "ok" | "err";
    text: string;
  } | null>(null);
  const [sel, setSel] = useState<Sel>({ kind: "general" });

  // 模型自动发现结果（针对当前选中的 provider）
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);

  const applySettings = (s: EditableSettings) => {
    setLoaded(s);
    setDrafts(toDrafts(s));
    setDefaultProvider(s.default_provider);
    setContextWindow(s.context_window);
    setMaxToolRounds(s.max_tool_rounds);
    setSystemPrompt(s.system_prompt ?? "");
    setSel({ kind: "general" });
  };

  useEffect(() => {
    getSettings()
      .then(applySettings)
      .catch((e: unknown) => {
        setLoadError(e instanceof Error ? e.message : String(e));
      });
  }, []);

  const refreshRuntime = () =>
    getRuntimeInfo()
      .then((i) => useStore.setState({ info: i, serverModel: i.model }))
      .catch(() => {});

  const patchDraft = (idx: number, patch: Partial<ProviderDraft>) =>
    setDrafts((ds) => ds.map((d, i) => (i === idx ? { ...d, ...patch } : d)));

  const renameDraft = (idx: number, name: string) => {
    if (drafts[idx]?.name.trim() === defaultProvider) {
      setDefaultProvider(name.trim());
    }
    patchDraft(idx, { name });
  };

  const addProvider = () => {
    setDrafts((ds) => [
      ...ds,
      {
        name: "",
        base_url: "",
        model: "",
        wire_api: "chat" as const,
        apiKeyInput: "",
        apiKeySet: false,
        max_tokens: null,
        temperature: null,
      },
    ]);
    setSel({ kind: "provider", index: drafts.length });
  };

  const removeProvider = (idx: number) => {
    setDrafts((ds) => ds.filter((_, i) => i !== idx));
    setSel((cur) => {
      if (cur.kind !== "provider") return cur;
      if (cur.index === idx)
        return { kind: "provider", index: Math.max(0, idx - 1) };
      if (cur.index > idx) return { kind: "provider", index: cur.index - 1 };
      return cur;
    });
  };

  /** 自动发现：探测 provider 的 OpenAI 兼容 `/models` 端点。 */
  const discoverModels = async (idx: number) => {
    const d = drafts[idx];
    if (!d || !d.base_url.trim()) {
      setModelsError(t("settings.baseUrlFirst"));
      return;
    }
    setModelsLoading(true);
    setModelsError(null);
    try {
      const res = await listModels({
        name: d.name.trim() || undefined,
        base_url: d.base_url.trim(),
        api_key: d.apiKeyInput.trim() || undefined,
        wire_api: d.wire_api,
      });
      setModelOptions(res.models);
      if (res.models.length === 0) setModelsError(t("settings.notDiscovered"));
    } catch (e) {
      setModelOptions([]);
      setModelsError(e instanceof Error ? e.message : String(e));
    } finally {
      setModelsLoading(false);
    }
  };

  // 切换到某 provider 时自动发现其模型（输入过程中不重复请求，可手动「自动发现」刷新）
  const providerIdx = sel.kind === "provider" ? sel.index : -1;
  useEffect(() => {
    if (providerIdx < 0) return;
    setModelOptions([]);
    setModelsError(null);
    const d = drafts[providerIdx];
    if (d && d.base_url.trim()) void discoverModels(providerIdx);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providerIdx]);

  const save = async () => {
    for (const d of drafts) {
      if (!d.name.trim()) {
        setSaveMsg({ kind: "err", text: t("settings.errProviderName") });
        return;
      }
      if (!d.base_url.trim()) {
        setSaveMsg({
          kind: "err",
          text: t("settings.errBaseUrl", { name: d.name }),
        });
        return;
      }
      if (!d.model.trim()) {
        setSaveMsg({
          kind: "err",
          text: t("settings.errModel", { name: d.name }),
        });
        return;
      }
    }
    if (!drafts.some((d) => d.name.trim() === defaultProvider)) {
      setSaveMsg({ kind: "err", text: t("settings.errDefaultNotInList") });
      return;
    }

    const payload: DraftSettings = {
      default_provider: defaultProvider,
      context_window: contextWindow,
      max_tool_rounds: maxToolRounds,
      system_prompt: systemPrompt.trim() ? systemPrompt : null,
      providers: drafts.map((d) => ({
        name: d.name.trim(),
        base_url: d.base_url.trim(),
        model: d.model.trim(),
        wire_api: d.wire_api,
        // 仅当用户输入了新值时携带；不携带 = 服务端保留已存密钥
        ...(d.apiKeyInput.trim() !== ""
          ? { api_key: d.apiKeyInput.trim() }
          : {}),
        max_tokens: d.max_tokens,
        temperature: d.temperature,
      })),
    };

    setSaving(true);
    setSaveMsg(null);
    try {
      const saved = await saveSettings(payload);
      applySettings(saved);
      await refreshRuntime();
      setSaveMsg({ kind: "ok", text: t("settings.saved") });
    } catch (e) {
      setSaveMsg({
        kind: "err",
        text: e instanceof Error ? e.message : String(e),
      });
    } finally {
      setSaving(false);
    }
  };

  const selected = sel.kind === "provider" ? drafts[sel.index] : undefined;

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* 子标题栏 */}
      <div className="h-11 shrink-0 border-b border-line flex items-center gap-3 px-4">
        <div className="text-sm font-medium">{t("settings.title")}</div>
        <span className="text-xs text-ink-muted">{t("settings.subtitle")}</span>
        <span className="flex-1" />
        <button
          type="button"
          onClick={closeSettings}
          className="text-xs text-ink-muted hover:text-ink border border-line rounded-md px-2.5 py-1"
        >
          {t("settings.backToChat")}
        </button>
      </div>

      {/* 三栏居中 */}
      <div className="flex-1 flex min-h-0 justify-center">
        <div className="w-full max-w-6xl flex min-h-0">
          {/* 左栏：分节导航 */}
          <nav className="w-56 shrink-0 border-r border-line overflow-y-auto py-2 text-xs">
            <div className="px-3 pb-1 text-[10px] uppercase tracking-wide text-ink-faint">
              {t("settings.section")}
            </div>
            <NavItem
              active={sel.kind === "general"}
              label={t("settings.general")}
              onClick={() => setSel({ kind: "general" })}
            />

            <div className="px-3 pt-3 pb-1 text-[10px] uppercase tracking-wide text-ink-faint">
              {t("settings.models")}
            </div>
            {loadError && (
              <div className="px-3 py-1 text-red-400 break-all">{loadError}</div>
            )}
            {!loaded && !loadError && (
              <div className="px-3 py-1 text-ink-faint">
                {t("settings.loading")}
              </div>
            )}
            {drafts.map((d, i) => (
              <NavItem
                key={i}
                active={sel.kind === "provider" && sel.index === i}
                label={d.name.trim() || t("settings.unnamed")}
                badge={
                  d.name.trim() === defaultProvider
                    ? t("settings.defaultBadge")
                    : undefined
                }
                onClick={() => setSel({ kind: "provider", index: i })}
              />
            ))}
            <button
              type="button"
              onClick={addProvider}
              className="w-full text-left px-3 py-1.5 text-ink-muted hover:text-ink"
            >
              {t("settings.addProvider")}
            </button>

            <div className="px-3 pt-3 pb-1 text-[10px] uppercase tracking-wide text-ink-faint">
              {t("settings.other")}
            </div>
            <NavItem
              active={sel.kind === "skills"}
              label={t("settings.skillsHeader", { n: info?.skills.length ?? 0 })}
              onClick={() => setSel({ kind: "skills" })}
            />
            <NavItem
              active={sel.kind === "identity"}
              label={t("settings.identity")}
              onClick={() => setSel({ kind: "identity" })}
            />
          </nav>

          {/* 中栏：选中分节内容 */}
          <div className="flex-1 min-w-0 overflow-y-auto p-5 text-sm space-y-7">
            {loadError ? (
              <div className="text-xs text-red-400 break-all">
                {t("settings.loadFailed", { err: loadError })}
              </div>
            ) : !loaded ? (
              <div className="text-ink-muted text-xs">{t("settings.loading")}</div>
            ) : sel.kind === "general" ? (
              <GeneralForm
                drafts={drafts}
                defaultProvider={defaultProvider}
                setDefaultProvider={setDefaultProvider}
                contextWindow={contextWindow}
                setContextWindow={setContextWindow}
                maxToolRounds={maxToolRounds}
                setMaxToolRounds={setMaxToolRounds}
              />
            ) : sel.kind === "provider" && selected ? (
              <ProviderForm
                draft={selected}
                isDefault={selected.name.trim() === defaultProvider}
                canDelete={drafts.length > 1}
                models={modelOptions}
                modelsLoading={modelsLoading}
                modelsError={modelsError}
                onDiscover={() => void discoverModels(sel.index)}
                onPatch={(patch) => patchDraft(sel.index, patch)}
                onRename={(name) => renameDraft(sel.index, name)}
                onSetDefault={() => setDefaultProvider(selected.name.trim())}
                onDelete={() => removeProvider(sel.index)}
              />
            ) : sel.kind === "skills" ? (
              <SkillsList info={info} />
            ) : (
              <IdentityForm
                systemPrompt={systemPrompt}
                setSystemPrompt={setSystemPrompt}
              />
            )}
          </div>

          {/* 右栏：保存 + 状态 */}
          <aside className="w-72 shrink-0 border-l border-line overflow-y-auto p-4 space-y-4">
            <button
              type="button"
              onClick={() => void save()}
              disabled={saving || !loaded}
              className="w-full bg-primary text-primary-fg hover:opacity-90 disabled:opacity-50 text-xs font-medium rounded-md py-2"
            >
              {saving ? t("settings.saving") : t("settings.saveApply")}
            </button>
            {saveMsg && (
              <div
                className={`text-xs ${
                  saveMsg.kind === "ok" ? "text-emerald-400" : "text-red-400"
                }`}
              >
                {saveMsg.text}
              </div>
            )}
            <p className="text-[11px] text-ink-faint leading-relaxed">
              {t("settings.saveHint")}
            </p>

            <div className="border-t border-line pt-4 space-y-2 text-xs">
              <div className="text-[10px] uppercase tracking-wide text-ink-faint">
                {t("settings.status")}
              </div>
              <StatusRow
                dot={wsConnected ? "ok" : "err"}
                label={
                  wsConnected ? t("top.connected") : t("top.reconnecting")
                }
              />
              <StatusRow
                label={t("settings.currentModel")}
                value={info?.model ?? serverModel ?? "—"}
              />
              <StatusRow
                label={t("settings.protocol")}
                value={info?.wire_api ?? "—"}
              />
              <StatusRow
                label={t("settings.workspace")}
                value={info?.cwd ?? "—"}
                mono
              />
            </div>

            <div className="border-t border-line pt-4 text-[11px] text-ink-muted leading-relaxed">
              {t("settings.configFile")}
              <div className="font-mono break-all text-ink-faint">
                {loaded?.config_path ?? "~/.sen-agent/config.toml"}
              </div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}

/* ------------------------------- 子组件 ------------------------------- */

function NavItem({
  active,
  label,
  badge,
  onClick,
}: {
  active: boolean;
  label: string;
  badge?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full flex items-center gap-2 text-left px-3 py-1.5 ${
        active ? "bg-hover text-ink" : "text-ink-muted hover:bg-elevated hover:text-ink"
      }`}
    >
      <span className="truncate">{label}</span>
      {badge && (
        <span className="ml-auto text-[10px] px-1.5 py-0.5 rounded bg-accent/15 text-accent">
          {badge}
        </span>
      )}
    </button>
  );
}

function GeneralForm({
  drafts,
  defaultProvider,
  setDefaultProvider,
  contextWindow,
  setContextWindow,
  maxToolRounds,
  setMaxToolRounds,
}: {
  drafts: ProviderDraft[];
  defaultProvider: string;
  setDefaultProvider: (v: string) => void;
  contextWindow: number;
  setContextWindow: (v: number) => void;
  maxToolRounds: number;
  setMaxToolRounds: (v: number) => void;
}) {
  const { t } = useI18n();
  return (
    <section className="max-w-2xl space-y-5">
      <h2 className="text-sm font-medium">{t("settings.generalHeader")}</h2>
      <label className="block">
        <div className="text-xs text-ink-muted mb-1">
          {t("settings.defaultProvider")}
        </div>
        <select
          value={defaultProvider}
          onChange={(e) => setDefaultProvider(e.target.value)}
          className={inputCls}
        >
          {drafts.map((d, i) => (
            <option key={i} value={d.name.trim()}>
              {d.name.trim() || t("settings.unnamed")}
            </option>
          ))}
        </select>
      </label>
      <div className="grid grid-cols-2 gap-3">
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.contextWindow")}
          </div>
          <input
            type="number"
            min={1024}
            value={contextWindow}
            onChange={(e) => setContextWindow(Number(e.target.value) || 0)}
            className={inputCls}
          />
        </label>
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.maxToolRounds")}
          </div>
          <input
            type="number"
            min={1}
            value={maxToolRounds}
            onChange={(e) => setMaxToolRounds(Number(e.target.value) || 0)}
            className={inputCls}
          />
        </label>
      </div>
    </section>
  );
}

function ProviderForm({
  draft,
  isDefault,
  canDelete,
  models,
  modelsLoading,
  modelsError,
  onDiscover,
  onPatch,
  onRename,
  onSetDefault,
  onDelete,
}: {
  draft: ProviderDraft;
  isDefault: boolean;
  canDelete: boolean;
  models: string[];
  modelsLoading: boolean;
  modelsError: string | null;
  onDiscover: () => void;
  onPatch: (patch: Partial<ProviderDraft>) => void;
  onRename: (name: string) => void;
  onSetDefault: () => void;
  onDelete: () => void;
}) {
  const { t } = useI18n();
  const modelValue = models.includes(draft.model) ? draft.model : "";
  return (
    <section className="max-w-2xl space-y-5">
      <div className="flex items-center gap-2">
        <h2 className="text-sm font-medium">{t("settings.models")}</h2>
        <span className="flex-1" />
        <button
          type="button"
          onClick={onSetDefault}
          disabled={isDefault}
          className="text-xs text-accent hover:opacity-80 disabled:opacity-40 disabled:text-ink-muted"
        >
          {isDefault ? t("settings.isDefault") : t("settings.setDefault")}
        </button>
        <button
          type="button"
          onClick={onDelete}
          disabled={!canDelete}
          className="text-xs text-ink-muted hover:text-red-400 disabled:opacity-30 disabled:hover:text-ink-muted"
        >
          {t("settings.delete")}
        </button>
      </div>

      <label className="block">
        <div className="text-xs text-ink-muted mb-1">
          {t("settings.providerName")}
        </div>
        <input
          value={draft.name}
          onChange={(e) => onRename(e.target.value)}
          placeholder={t("settings.providerNamePlaceholder")}
          className={inputCls}
        />
      </label>
      <label className="block">
        <div className="text-xs text-ink-muted mb-1">{t("settings.baseUrl")}</div>
        <input
          value={draft.base_url}
          onChange={(e) => onPatch({ base_url: e.target.value })}
          placeholder="https://api.openai.com/v1"
          className={inputCls}
        />
      </label>

      {/* 模型：自动发现 + 下拉切换（含手动填写） */}
      <div>
        <div className="flex items-center gap-2 mb-1">
          <div className="text-xs text-ink-muted">{t("settings.model")}</div>
          <button
            type="button"
            onClick={onDiscover}
            disabled={modelsLoading}
            className="text-[11px] text-accent hover:opacity-80 disabled:opacity-50"
          >
            {modelsLoading ? t("settings.discovering") : t("settings.discover")}
          </button>
        </div>
        <select
          value={modelValue}
          onChange={(e) => e.target.value && onPatch({ model: e.target.value })}
          className={inputCls}
        >
          <option value="">
            {models.length
              ? t("settings.modelFromDiscovery")
              : t("settings.modelManualHint")}
          </option>
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <input
          value={draft.model}
          onChange={(e) => onPatch({ model: e.target.value })}
          placeholder={t("settings.modelManualPlaceholder")}
          className={`${inputCls} mt-2`}
        />
        {modelsError && (
          <div className="text-[11px] text-red-400 mt-1">{modelsError}</div>
        )}
        {!modelsError && models.length > 0 && (
          <div className="text-[11px] text-ink-faint mt-1">
            {t("settings.discoveredCount", { n: models.length })}
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.wireApi")}
          </div>
          <select
            value={draft.wire_api}
            onChange={(e) =>
              onPatch({ wire_api: e.target.value as ProviderDraft["wire_api"] })
            }
            className={inputCls}
          >
            <option value="chat">{t("settings.wireApiChat")}</option>
            <option value="responses">{t("settings.wireApiResponses")}</option>
          </select>
        </label>
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.apiKey")}
          </div>
          <input
            type={
              draft.apiKeyInput.startsWith("env:") || !draft.apiKeySet
                ? "text"
                : "password"
            }
            value={draft.apiKeyInput}
            onChange={(e) => onPatch({ apiKeyInput: e.target.value })}
            placeholder={
              draft.apiKeySet
                ? t("settings.apiKeySetPlaceholder")
                : t("settings.apiKeyPlaceholder")
            }
            className={inputCls}
          />
        </label>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.maxTokens")}
          </div>
          <input
            type="number"
            min={1}
            value={draft.max_tokens ?? ""}
            onChange={(e) =>
              onPatch({
                max_tokens: e.target.value === "" ? null : Number(e.target.value),
              })
            }
            placeholder={t("settings.maxTokensPlaceholder")}
            className={inputCls}
          />
        </label>
        <label className="block">
          <div className="text-xs text-ink-muted mb-1">
            {t("settings.temperature")}
          </div>
          <input
            type="number"
            step="0.1"
            min={0}
            max={2}
            value={draft.temperature ?? ""}
            onChange={(e) =>
              onPatch({
                temperature:
                  e.target.value === "" ? null : Number(e.target.value),
              })
            }
            placeholder={t("settings.temperaturePlaceholder")}
            className={inputCls}
          />
        </label>
      </div>
    </section>
  );
}

function IdentityForm({
  systemPrompt,
  setSystemPrompt,
}: {
  systemPrompt: string;
  setSystemPrompt: (v: string) => void;
}) {
  const { t } = useI18n();
  return (
    <section className="max-w-2xl space-y-5">
      <h2 className="text-sm font-medium">{t("settings.identityHeader")}</h2>
      <p className="text-[11px] text-ink-faint leading-relaxed">
        {t("settings.identityHint")}
      </p>
      <label className="block">
        <div className="text-xs text-ink-muted mb-1">
          {t("settings.systemPrompt")}
        </div>
        <textarea
          value={systemPrompt}
          onChange={(e) => setSystemPrompt(e.target.value)}
          rows={12}
          placeholder={t("settings.systemPromptPlaceholder")}
          className={`${inputCls} resize-y`}
        />
      </label>
    </section>
  );
}

function SkillsList({ info }: { info: RuntimeInfo | null }) {
  const { t } = useI18n();
  const skills = info?.skills ?? [];
  return (
    <section className="max-w-2xl space-y-4">
      <h2 className="text-sm font-medium">
        {t("settings.skillsHeader", { n: skills.length })}
      </h2>
      {skills.length === 0 ? (
        <div className="text-xs text-ink-faint">{t("settings.noSkills")}</div>
      ) : (
        <div className="space-y-3">
          {skills.map((s) => (
            <div
              key={`${s.group}/${s.name}`}
              className="border border-line rounded-md p-3"
            >
              <div className="text-xs font-medium text-ink font-mono">
                {s.group ? `${s.group}/` : ""}
                {s.name}
              </div>
              <div className="text-[11px] text-ink-muted mt-1">
                {s.description}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function StatusRow({
  dot,
  label,
  value,
  mono,
}: {
  dot?: "ok" | "err";
  label: string;
  value?: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center gap-2">
      {dot && (
        <span
          className={`w-2 h-2 rounded-full ${dot === "ok" ? "bg-emerald-400" : "bg-red-400"}`}
        />
      )}
      <span className="text-ink-muted">{label}</span>
      {value !== undefined && (
        <span
          className={`ml-auto text-ink break-all text-right ${mono ? "font-mono text-[10px]" : ""}`}
        >
          {value}
        </span>
      )}
    </div>
  );
}
