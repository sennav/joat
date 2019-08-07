use clap::{App, Shell};
use std::fs;

use crate::{template, Context};

pub fn execute_init(context: Context) {
    let init_template = String::from(include_str!("../templates/config_template.yml"));
    let yaml_str = template::get_compiled_template_str_with_context(&init_template, &context)
        .expect("Could not create yaml template");
    let cmd_name = &context["args"]["PROJECT_NAME"];
    let filename = format!("{}.yml", cmd_name);
    fs::write(filename, yaml_str).expect("Unable to write file");
    println!("Config file {}.yml created", cmd_name);
    println!("To start testing with your extension create a symlink in your PATH targeting joat binaries with name: {}", cmd_name);
}

pub fn execute_auto_complete(mut app: App, app_name: &str, context: Context) {
    let selected_shell = &context["args"]["SHELL"];
    let shell;
    match selected_shell.as_ref() {
        "zsh" => shell = Shell::Zsh,
        "bash" => shell = Shell::Bash,
        "fish" => shell = Shell::Fish,
        "powershell" => shell = Shell::PowerShell,
        "elvish" => shell = Shell::Elvish,
        _ => panic!("Unknown shell, use only lowercase"),
    };
    app.gen_completions(app_name, shell, ".")
}
