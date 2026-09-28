use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

const SERVER_BINARY: &str = "kage-ls";

struct KageExtension;

impl zed::Extension for KageExtension {
    fn new() -> Self {
        KageExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let path = worktree.which(SERVER_BINARY).unwrap_or_else(|| {
            eprintln!(
                "`kage-ls` was not found on PATH. Install it with `cargo install --path crates/kage-ls`."
            );
            SERVER_BINARY.to_string()
        });
        Ok(Command {
            command: path,
            args: vec![],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(KageExtension);
