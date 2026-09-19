//! `lokey <command> [key=NAME] [value=VALUE] [project=NAME] [--flag]`
//!
//! Hand-parsed: there are a dozen commands and three named arguments, which a
//! parsing library would not make shorter. Named arguments may come in any
//! order. `key="NAME"` reaches us as `key=NAME` from PowerShell, cmd and bash.

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub command: String,
    pub key: Option<String>,
    pub value: Option<String>,
    pub project: Option<String>,
    pub view: bool,
    /// Positional operand, used only by the hidden clipboard-clearing helper.
    pub operand: Option<String>,
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut args = Args::default();
    for token in argv {
        if let Some(flag) = token.strip_prefix("--") {
            match flag.to_lowercase().as_str() {
                "view" => args.view = true,
                "help" | "version" if args.command.is_empty() => args.command = flag.to_lowercase(),
                _ => return Err(format!("unknown option '--{flag}'")),
            }
            continue;
        }
        if let Some((name, value)) = token.split_once('=') {
            let slot = match name.to_lowercase().as_str() {
                "key" => &mut args.key,
                "value" => &mut args.value,
                "project" => &mut args.project,
                _ => {
                    return Err(format!(
                        "unknown argument '{name}='; expected key=, value= or project="
                    ));
                }
            };
            if slot.is_some() {
                return Err(format!("'{name}=' given twice"));
            }
            *slot = Some(value.to_string());
            continue;
        }
        if args.command.is_empty() {
            args.command = token.to_lowercase();
        } else if args.operand.is_none() {
            args.operand = Some(token.clone());
        } else {
            return Err(format!("unexpected '{token}'"));
        }
    }
    if args.command == "-h" {
        args.command = "help".into();
    } else if args.command == "-v" {
        args.command = "version".into();
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parse_reads_named_arguments_in_any_order() {
        let args = parse(&argv(&["set", "value=abc", "key=API_KEY", "project=web"])).unwrap();

        assert_eq!(
            args,
            Args {
                command: "set".into(),
                key: Some("API_KEY".into()),
                value: Some("abc".into()),
                project: Some("web".into()),
                ..Args::default()
            }
        );
    }

    #[test]
    fn parse_keeps_equals_signs_inside_the_value() {
        let args = parse(&argv(&["set", "key=URL", "value=https://h/?a=1&b=2"])).unwrap();

        assert_eq!(args.value.as_deref(), Some("https://h/?a=1&b=2"));
    }

    #[test]
    fn parse_lowercases_the_command_so_lokey_get_works() {
        let args = parse(&argv(&["GET", "key=A"])).unwrap();

        assert_eq!(args.command, "get");
    }

    #[test]
    fn parse_reads_view_flag() {
        let args = parse(&argv(&["get", "key=A", "--view"])).unwrap();

        assert!(args.view);
    }

    #[test]
    fn parse_rejects_unknown_named_argument() {
        let result = parse(&argv(&["set", "name=A"]));

        assert!(result.is_err());
    }

    #[test]
    fn parse_rejects_repeated_argument() {
        let result = parse(&argv(&["set", "key=A", "key=B"]));

        assert!(result.is_err());
    }

    #[test]
    fn parse_accepts_empty_value() {
        let args = parse(&argv(&["set", "key=EMPTY", "value="])).unwrap();

        assert_eq!(args.value.as_deref(), Some(""));
    }
}
