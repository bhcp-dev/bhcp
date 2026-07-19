#[cfg(target_os = "linux")]
mod linux {

    use std::env;
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitCode};

    use landlock::{
        ABI, Access, AccessFs, BitFlags, CompatLevel, Compatible, Ruleset, RulesetAttr,
        RulesetCreatedAttr, RulesetStatus, path_beneath_rules,
    };

    const NETWORK_FILTER: &str = r#"{
  "adapter": {
    "mismatch_action": "allow",
    "match_action": { "errno": 1 },
    "filter": [
      { "syscall": "socket" },
      { "syscall": "socketpair" },
      { "syscall": "connect" },
      { "syscall": "bind" },
      { "syscall": "listen" },
      { "syscall": "accept" },
      { "syscall": "accept4" },
      { "syscall": "sendto" },
      { "syscall": "sendmsg" },
      { "syscall": "sendmmsg" },
      { "syscall": "recvfrom" },
      { "syscall": "recvmsg" },
      { "syscall": "recvmmsg" },
      { "syscall": "shutdown" },
      { "syscall": "getsockname" },
      { "syscall": "getpeername" },
      { "syscall": "setsockopt" },
      { "syscall": "getsockopt" },
      { "syscall": "io_uring_setup" }
    ]
  }
}"#;

    pub fn main() -> ExitCode {
        match run() {
            Ok(never) => never,
            Err(message) => {
                eprintln!("BHCP adapter sandbox: {message}");
                ExitCode::from(125)
            }
        }
    }

    fn run() -> Result<ExitCode, String> {
        let mut arguments = env::args_os().skip(1);
        if arguments.next().as_deref() != Some("--project-root".as_ref()) {
            return Err("missing --project-root".to_owned());
        }
        let project_root = arguments
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "missing project root".to_owned())?;
        if arguments.next().as_deref() != Some("--effects".as_ref()) {
            return Err("missing --effects".to_owned());
        }
        let effects = arguments
            .next()
            .ok_or_else(|| "missing effect set".to_owned())?
            .into_string()
            .map_err(|_| "effect set is not UTF-8".to_owned())?;
        if arguments.next().as_deref() != Some("--".as_ref()) {
            return Err("missing command separator".to_owned());
        }
        let executable = arguments
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "missing executable".to_owned())?;

        restrict_filesystem(&project_root, &executable, &effects)?;
        restrict_network()?;

        let error = Command::new(&executable).args(arguments).exec();
        Err(format!("cannot execute registered adapter: {error}"))
    }

    fn restrict_filesystem(
        project_root: &Path,
        executable: &Path,
        effects: &str,
    ) -> Result<(), String> {
        let abi = ABI::V4;
        let mut ruleset = Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessFs::from_all(abi))
            .map_err(|error| format!("cannot define filesystem restrictions: {error}"))?
            .create()
            .map_err(|error| format!("cannot create filesystem restrictions: {error}"))?;

        let system_paths = ["/usr", "/lib", "/lib64", "/etc/ld.so.cache", "/dev/null"]
            .into_iter()
            .filter(|path| Path::new(path).exists());
        let data_read = AccessFs::from_read(abi) & !AccessFs::Execute;
        ruleset = ruleset
            .add_rules(path_beneath_rules(system_paths, data_read))
            .map_err(|error| format!("cannot grant platform runtime reads: {error}"))?;
        let loaders = [
            "/lib64/ld-linux-x86-64.so.2",
            "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
            "/lib/ld-linux-aarch64.so.1",
            "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1",
        ]
        .into_iter()
        .filter(|path| Path::new(path).exists());
        ruleset = ruleset
            .add_rules(path_beneath_rules(loaders, AccessFs::from_read(abi)))
            .map_err(|error| format!("cannot grant dynamic-loader execution: {error}"))?;
        ruleset = ruleset
            .add_rules(path_beneath_rules([executable], AccessFs::from_read(abi)))
            .map_err(|error| format!("cannot grant executable read: {error}"))?;

        let read = effects
            .split(',')
            .any(|effect| effect == "bhcp-effect/fs.read@0");
        let write = effects
            .split(',')
            .any(|effect| effect == "bhcp-effect/fs.write@0");
        if read || write {
            let mut project_access = BitFlags::<AccessFs>::EMPTY;
            if read {
                project_access |= data_read;
            }
            if write {
                project_access |= AccessFs::from_write(abi);
            }
            ruleset = ruleset
                .add_rules(path_beneath_rules([project_root], project_access))
                .map_err(|error| format!("cannot grant project filesystem scope: {error}"))?;
        }

        let status = ruleset
            .restrict_self()
            .map_err(|error| format!("cannot install filesystem restrictions: {error}"))?;
        if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
            return Err(format!(
                "filesystem restrictions were not fully enforced: {status:?}"
            ));
        }
        Ok(())
    }

    fn restrict_network() -> Result<(), String> {
        let architecture = env::consts::ARCH
            .try_into()
            .map_err(|_| format!("unsupported seccomp architecture {}", env::consts::ARCH))?;
        let filters = seccompiler::compile_from_json(NETWORK_FILTER.as_bytes(), architecture)
            .map_err(|error| format!("cannot compile network restrictions: {error}"))?;
        let filter = filters
            .get("adapter")
            .ok_or_else(|| "compiled network restriction is missing".to_owned())?;
        seccompiler::apply_filter(filter)
            .map_err(|error| format!("cannot install network restrictions: {error}"))
    }
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    linux::main()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("BHCP adapter sandbox: this helper is only supported on Linux");
}
