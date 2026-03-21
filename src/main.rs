mod app;
mod config;
mod game;
mod render;

use std::env;

use anyhow::{Result, bail};

use crate::app::App;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CliOptions {
    no_color: bool,
    test_ai: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliAction {
    Run(CliOptions),
    Help,
}

/// 启动应用并进入主游戏循环。
fn main() -> Result<()> {
    let mut args = env::args();
    let program_name = args.next().unwrap_or_else(|| String::from("rust-snake"));

    match parse_args(args, &program_name)? {
        CliAction::Run(options) => App::new(options.no_color, options.test_ai).run(),
        CliAction::Help => {
            println!("{}", help_text(&program_name));
            Ok(())
        }
    }
}

/// 解析命令行参数。
fn parse_args<I>(args: I, program_name: &str) -> Result<CliAction>
where
    I: IntoIterator<Item = String>,
{
    let mut options = CliOptions::default();

    for arg in args {
        match arg.as_str() {
            "--no-color" => options.no_color = true,
            "--test-ai" => options.test_ai = true,
            "--help" | "-h" => return Ok(CliAction::Help),
            _ => bail!("未知参数：{arg}\n\n{}", help_text(program_name)),
        }
    }

    Ok(CliAction::Run(options))
}

/// 返回命令行帮助文本。
fn help_text(program_name: &str) -> String {
    format!(
        "\
{program_name}

Usage:
  {program_name} [OPTIONS]

Options:
  --no-color  禁用终端颜色输出
  --test-ai   让玩家蛇也由 AI 控制
  -h, --help  显示帮助信息
"
    )
}

#[cfg(test)]
mod tests {
    use super::{CliAction, CliOptions, help_text, parse_args};

    #[test]
    /// 验证长选项会被正确解析为运行配置。
    fn long_options_are_parsed_into_cli_options() {
        let action = parse_args(
            [String::from("--no-color"), String::from("--test-ai")],
            "rust-snake",
        )
        .unwrap();

        assert_eq!(
            action,
            CliAction::Run(CliOptions {
                no_color: true,
                test_ai: true,
            })
        );
    }

    #[test]
    /// 验证 help 选项会直接返回帮助动作。
    fn help_option_returns_help_action() {
        let action = parse_args([String::from("--help")], "rust-snake").unwrap();

        assert_eq!(action, CliAction::Help);
    }

    #[test]
    /// 验证未知参数会返回错误并附带帮助文本。
    fn unknown_option_returns_helpful_error() {
        let error = parse_args([String::from("--unknown")], "rust-snake").unwrap_err();
        let message = error.to_string();

        assert!(message.contains("未知参数：--unknown"));
        assert!(message.contains(&help_text("rust-snake")));
    }
}
