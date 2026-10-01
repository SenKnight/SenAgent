/** 设置面板：运行时信息（模型 / 协议 / 工作目录 / 已识别技能）。 */

import { getRuntimeInfo } from "../api";
import { useStore } from "../store";

export function SettingsPanel() {
  const info = useStore((s) => s.info);
  const wsConnected = useStore((s) => s.wsConnected);
  const serverModel = useStore((s) => s.serverModel);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  const refresh = () => {
    void getRuntimeInfo().then((i) => useStore.setState({ info: i }));
  };

  return (
    <aside className="w-80 shrink-0 border-l border-zinc-800 bg-zinc-950 flex flex-col">
      <div className="p-3 border-b border-zinc-800 flex items-center justify-between">
        <div className="text-sm font-medium">设置与信息</div>
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
            <div className="text-xs text-zinc-500 mt-1">服务端模型: {serverModel}</div>
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
            onClick={refresh}
            className="mt-3 text-xs text-blue-400 hover:text-blue-300"
          >
            刷新
          </button>
        </section>

        <section>
          <div className="text-xs text-zinc-500 mb-2">
            已识别技能（~/.agents/skills）
          </div>
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
          模型与 API Key 等配置位于{" "}
          <code className="text-zinc-400">~/.sen-agent/config.toml</code>
          ，修改后重启服务生效。指导提示词支持全局{" "}
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
