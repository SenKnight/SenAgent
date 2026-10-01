/** 设置面板：连接状态 + 交互式模型配置（保存即热生效）+ 技能与运行时信息。 */

import { useEffect, useState } from "react";

import { getRuntimeInfo, getSettings, saveSettings } from "../api";
import { useStore } from "../store";
import type { DraftSettings, EditableSettings } from "../types";

/** 表单内 Provider 草稿：区分服务端回显与用户新输入。 */
interface ProviderDraft {
  name: string;
  base_url: string;
  model: string;
  wire_api: "chat" | "responses";
  /** 用户新输入；空串 = 不改动（保留服务端已存密钥）。env: 引用直接回显在输入框。 */
  apiKeyInput: string;
  /** 服务端已存密钥标记（明文不回显）。 */
  apiKeySet: boolean;
  max_tokens: number | null;
  temperature: number | null;
}

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
  "w-full bg-zinc-900 border border-zinc-800 rounded-md px-2 py-1.5 text-xs " +
  "text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-zinc-500";

export function SettingsPanel() {
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  const [loaded, setLoaded] = useState<EditableSettings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<ProviderDraft[]>([]);
  const [defaultProvider, setDefaultProvider] = useState("");
  const [contextWindow, setContextWindow] = useState(128000);
  const [maxToolRounds, setMaxToolRounds] = useState(25);
  const [systemPrompt, setSystemPrompt] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<{ kind: "ok" | "err"; text: string } | null>(null);

  const applySettings = (s: EditableSettings) => {
    setLoaded(s);
    setDrafts(toDrafts(s));
    setDefaultProvider(s.default_provider);
    setContextWindow(s.context_window);
    setMaxToolRounds(s.max_tool_rounds);
    setSystemPrompt(s.system_prompt ?? "");
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

  const addProvider = () =>
    setDrafts((ds) => [
      ...ds,
      {
        name: "",
        base_url: "",
        model: "",
        wire_api: "chat",
        apiKeyInput: "",
        apiKeySet: false,
        max_tokens: null,
        temperature: null,
      },
    ]);

  const removeProvider = (idx: number) =>
    setDrafts((ds) => ds.filter((_, i) => i !== idx));

  const save = async () => {
    for (const d of drafts) {
      if (!d.name.trim()) {
        setSaveMsg({ kind: "err", text: "provider 名称不能为空" });
        return;
      }
      if (!d.base_url.trim()) {
        setSaveMsg({ kind: "err", text: `「${d.name}」的 Base URL 不能为空` });
        return;
      }
      if (!d.model.trim()) {
        setSaveMsg({ kind: "err", text: `「${d.name}」的模型不能为空` });
        return;
      }
    }
    if (!drafts.some((d) => d.name.trim() === defaultProvider)) {
      setSaveMsg({ kind: "err", text: "默认 Provider 不在列表中" });
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
        ...(d.apiKeyInput.trim() !== "" ? { api_key: d.apiKeyInput.trim() } : {}),
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
      setSaveMsg({ kind: "ok", text: "已保存并生效，下一条消息即使用新配置" });
    } catch (e) {
      setSaveMsg({ kind: "err", text: e instanceof Error ? e.message : String(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <aside className="w-96 shrink-0 border-l border-zinc-800 bg-zinc-950 flex flex-col">
      <div className="p-3 border-b border-zinc-800 flex items-center justify-between">
        <div className="text-sm font-medium">设置</div>
        <button
          type="button"
          onClick={() => setSettingsOpen(false)}
          className="text-zinc-500 hover:text-zinc-200 text-sm"
        >
          关闭
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-5 text-sm">
        <section>
          <div className="text-xs text-zinc-500 mb-2">连接状态</div>
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                wsConnected ? "bg-emerald-400" : "bg-red-400"
              }`}
            />
            <span>{wsConnected ? "已连接" : "未连接（自动重连中）"}</span>
          </div>
          {serverModel && (
            <div className="text-xs text-zinc-500 mt-1">当前模型: {serverModel}</div>
          )}
        </section>

        <section>
          <div className="text-xs text-zinc-500 mb-2">模型与服务（保存后立即生效）</div>
          {loadError ? (
            <div className="text-xs text-red-400 break-all">加载配置失败: {loadError}</div>
          ) : !loaded ? (
            <div className="text-xs text-zinc-500">加载中…</div>
          ) : (
            <div className="space-y-3">
              <div>
                <div className="text-xs text-zinc-500 mb-1">默认 Provider</div>
                <select
                  value={defaultProvider}
                  onChange={(e) => setDefaultProvider(e.target.value)}
                  className={inputCls}
                >
                  {drafts.map((d, i) => (
                    <option key={i} value={d.name.trim()}>
                      {d.name.trim() || "(未命名)"}
                    </option>
                  ))}
                </select>
              </div>

              {drafts.map((d, i) => (
                <div key={i} className="border border-zinc-800 rounded-lg p-3 space-y-2">
                  <div className="flex items-center gap-2">
                    <input
                      value={d.name}
                      onChange={(e) => renameDraft(i, e.target.value)}
                      placeholder="名称，如 openai / deepseek / ollama"
                      className="flex-1 bg-zinc-900 border border-zinc-800 rounded-md px-2 py-1.5 text-xs font-medium text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-zinc-500"
                    />
                    <button
                      type="button"
                      onClick={() => removeProvider(i)}
                      disabled={drafts.length <= 1}
                      className="text-zinc-500 hover:text-red-400 text-xs shrink-0 disabled:opacity-30 disabled:hover:text-zinc-500"
                    >
                      删除
                    </button>
                  </div>
                  <input
                    value={d.base_url}
                    onChange={(e) => patchDraft(i, { base_url: e.target.value })}
                    placeholder="Base URL（https://api.openai.com/v1）"
                    className={inputCls}
                  />
                  <input
                    value={d.model}
                    onChange={(e) => patchDraft(i, { model: e.target.value })}
                    placeholder="模型（gpt-5 / deepseek-chat / qwen3:8b）"
                    className={inputCls}
                  />
                  <div className="flex gap-2">
                    <select
                      value={d.wire_api}
                      onChange={(e) =>
                        patchDraft(i, { wire_api: e.target.value as ProviderDraft["wire_api"] })
                      }
                      className="w-36 bg-zinc-900 border border-zinc-800 rounded-md px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-zinc-500"
                    >
                      <option value="chat">chat 协议</option>
                      <option value="responses">responses 协议</option>
                    </select>
                  </div>
                  <input
                    type={
                      d.apiKeyInput.startsWith("env:") || !d.apiKeySet ? "text" : "password"
                    }
                    value={d.apiKeyInput}
                    onChange={(e) => patchDraft(i, { apiKeyInput: e.target.value })}
                    placeholder={
                      d.apiKeySet
                        ? "已保存密钥，留空保持不变（输入新值可替换）"
                        : "API Key（可填 env:VAR_NAME 引用环境变量；可留空）"
                    }
                    className={inputCls}
                  />
                </div>
              ))}

              <button
                type="button"
                onClick={addProvider}
                className="w-full border border-dashed border-zinc-700 rounded-lg py-1.5 text-xs text-zinc-400 hover:text-zinc-200 hover:border-zinc-500"
              >
                + 添加 Provider
              </button>

              <div className="grid grid-cols-2 gap-2">
                <label className="block">
                  <div className="text-xs text-zinc-500 mb-1">上下文窗口（token）</div>
                  <input
                    type="number"
                    min={1024}
                    value={contextWindow}
                    onChange={(e) => setContextWindow(Number(e.target.value) || 0)}
                    className={inputCls}
                  />
                </label>
                <label className="block">
                  <div className="text-xs text-zinc-500 mb-1">工具轮数上限</div>
                  <input
                    type="number"
                    min={1}
                    value={maxToolRounds}
                    onChange={(e) => setMaxToolRounds(Number(e.target.value) || 0)}
                    className={inputCls}
                  />
                </label>
              </div>

              <label className="block">
                <div className="text-xs text-zinc-500 mb-1">追加系统提示词（可选）</div>
                <textarea
                  value={systemPrompt}
                  onChange={(e) => setSystemPrompt(e.target.value)}
                  rows={3}
                  placeholder="例如：始终使用中文回答，代码注释用中文"
                  className={`${inputCls} resize-y`}
                />
              </label>

              <button
                type="button"
                onClick={() => void save()}
                disabled={saving}
                className="w-full bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white text-xs font-medium rounded-md py-2"
              >
                {saving ? "保存中…" : "保存并生效"}
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
            </div>
          )}
        </section>

        <section>
          <div className="text-xs text-zinc-500 mb-2">运行时配置</div>
          {info ? (
            <dl className="space-y-1.5">
              <InfoRow label="Provider" value={info.provider} />
              <InfoRow label="模型" value={info.model} />
              <InfoRow label="协议" value={info.wire_api} />
              <InfoRow label="工作目录" value={info.cwd} mono />
            </dl>
          ) : (
            <div className="text-zinc-500">加载中…</div>
          )}
          <button
            type="button"
            onClick={() => void refreshRuntime()}
            className="mt-3 text-xs text-blue-400 hover:text-blue-300"
          >
            刷新
          </button>
        </section>

        <section>
          <div className="text-xs text-zinc-500 mb-2">已识别技能（~/.agents/skills）</div>
          {info && info.skills.length > 0 ? (
            <ul className="space-y-1.5">
              {info.skills.map((s) => (
                <li key={`${s.group}/${s.name}`} className="text-xs">
                  <span className="text-zinc-200 font-mono">
                    {s.group ? `${s.group}/` : ""}
                    {s.name}
                  </span>
                  <div className="text-zinc-500">{s.description}</div>
                </li>
              ))}
            </ul>
          ) : (
            <div className="text-xs text-zinc-500">
              暂无技能。可在对话中让 Agent 创建，或运行{" "}
              <code className="text-zinc-400">sen skill new &lt;分组/名称&gt;</code>
            </div>
          )}
        </section>

        <section className="text-xs text-zinc-500 leading-relaxed">
          配置保存于{" "}
          <code className="text-zinc-400 break-all">
            {loaded?.config_path ?? "~/.sen-agent/config.toml"}
          </code>
          ，页面保存后立即生效，无需手动编辑文件或重启。指导提示词支持全局{" "}
          <code className="text-zinc-400">~/.agents/AGENTS.md</code> 与项目级{" "}
          <code className="text-zinc-400">AGENTS.md</code>（从工作目录向上查找）。
        </section>
      </div>
    </aside>
  );
}

function InfoRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex gap-2">
      <dt className="text-zinc-500 w-20 shrink-0">{label}</dt>
      <dd className={`text-zinc-200 break-all ${mono ? "font-mono text-xs" : ""}`}>
        {value}
      </dd>
    </div>
  );
}
