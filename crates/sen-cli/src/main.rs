//! sen：SenAgent CLI（进程内直连内核，不经过 HTTP）。

mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// SenAgent：跨平台个人 AI Agent
#[derive(Parser)]
#[command(
    name = "sen",
    version,
    about = "SenAgent: 跨平台个人 AI Agent（CLI / Web / 桌面共用同一内核）"
)]
struct Cli {
    /// 指定 provider（覆盖配置中的 default_provider）
    #[arg(long, global = true)]
    provider: Option<String>,

    /// 继续指定会话（完整 id 或唯一前缀）
    #[arg(long, global = true)]
    session: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 交互式对话（流式 REPL）
    Chat,
    /// 单次问答
    Run {
        /// 问题内容（可多段，自动拼接）
        #[arg(required = true, trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// 会话管理（默认列出全部会话）
    Sessions {
        #[command(subcommand)]
        cmd: Option<SessionsCmd>,
    },
    /// 配置管理（默认显示当前配置）
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCmd>,
    },
    /// 技能管理（~/.agents/skills）
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
    },
    /// 启动 Web 服务（浏览器与桌面端共用同一入口）
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8642)]
        port: u16,
        /// 前端构建产物目录（默认自动探测 ./frontend/dist）
        #[arg(long)]
        static_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SessionsCmd {
    /// 列出全部会话
    List,
    /// 显示会话的完整消息
    Show { id: String },
    /// 重命名会话
    Rename {
        id: String,
        #[arg(required = true)]
        title: Vec<String>,
    },
    /// 删除会话
    Delete { id: String },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 显示当前配置
    Show,
    /// （重新）生成模板配置
    Init,
    /// 输出配置文件路径
    Path,
}

#[derive(Subcommand)]
enum SkillCmd {
    /// 列出全部技能（按分组展示）
    List,
    /// 显示技能全文
    Show { name: String },
    /// 创建技能（如 `sen skill new coding/git-helper`）
    New {
        /// 相对 ~/.agents/skills 的技能目录，支持多级分组
        path: String,
        /// 技能名称（默认取目录最后一段）
        #[arg(long)]
        name: Option<String>,
        /// 触发场景的一句话描述
        #[arg(long)]
        description: Option<String>,
        /// 从文件读取 SKILL.md 正文
        #[arg(long)]
        content_file: Option<PathBuf>,
    },
    /// 输出技能根目录路径
    Dir,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("sen_core=info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Chat => commands::repl::run(cli.provider, cli.session).await,
        Command::Run { prompt } => {
            commands::run_once(cli.provider, cli.session, prompt.join(" ")).await
        }
        Command::Sessions { cmd } => commands::sessions::run(cmd).await,
        Command::Config { cmd } => commands::config_cmd::run(cmd),
        Command::Skill { cmd } => commands::skill::run(cmd).await,
        Command::Serve {
            host,
            port,
            static_dir,
        } => commands::serve::run(cli.provider, host, port, static_dir).await,
    }
}
