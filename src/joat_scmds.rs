use clap::{App, Shell};
use std::fs;

use crate::{template, Context};

pub fn execute_init(context: Context) {
    let init_template = String::from(include_str!("../templates/config_template.yml"));
    let yaml_str = template::get_compiled_template_str_with_context(&init_template, &context)
        .expect("Could not create yaml template");
    let raw_cmd_name = &context["args"]["PROJECT_NAME"];
    let cmd_name = raw_cmd_name.as_str().expect("Project is not string");
    let filename = format!("{}.yml", cmd_name);
    fs::write(filename, yaml_str).expect("Unable to write file");
    println!("Config file {}.yml created", cmd_name);
    println!("To start testing with your extension create a symlink in your PATH targeting joat binaries with name: {}", cmd_name);
}

pub fn execute_auto_complete(mut app: App, app_name: &str, context: Context) {
    let selected_shell = &context["args"]["SHELL"]
        .as_str()
        .expect("Could not convert shell argument to string");
    let shell;
    let lower_selected_shell = selected_shell.to_string().to_lowercase();
    match lower_selected_shell.as_str() {
        "zsh" => shell = Shell::Zsh,
        "bash" => shell = Shell::Bash,
        "fish" => shell = Shell::Fish,
        "powershell" => shell = Shell::PowerShell,
        "elvish" => shell = Shell::Elvish,
        shell => {
            eprintln!("Shell not supported {}. Options are:", shell);
            eprintln!("zsh");
            eprintln!("bash");
            eprintln!("fish");
            eprintln!("powershell");
            eprintln!("elvish");
            ::std::process::exit(1);
        }
    };
    app.gen_completions(app_name, shell, ".")
}
