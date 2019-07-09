extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;
extern crate regex;
extern crate dirs;

use clap::{
    App,
    ArgMatches
};
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::env;
use regex::Regex;

mod template;
mod http;
mod yaml;
mod oauth;
mod script_scmd;
mod request_scmd;
mod joat_scmds;

fn get_args_context(args: &ArgMatches, subcmd_yaml: &Yaml) -> HashMap<String, String> {
    let mut args_context = HashMap::new();
    for arg in subcmd_yaml["args"].clone().into_iter() {
        for a in arg.into_hash().unwrap().keys() {
            let key = a.clone().into_string().unwrap();
            if args.is_present(&key) {
                let value = args.value_of(&key).unwrap_or("");
                args_context.insert(key, String::from(value));
            }
        }
    }
    return args_context;
}

fn get_vars_context(yaml: &Yaml) -> HashMap<String, String> {
    let mut vars_context = HashMap::new();
    let vars_yaml = &yaml["vars"];
    if !vars_yaml.is_badvalue() {
        let vars_iter = vars_yaml
            .clone()
            .into_hash()
            .expect("Could not convert vars into hash");
        for (key, value) in vars_iter {
            let key_str = key.clone().into_string().expect("Var key should be string");
            let value_str = value.clone().into_string().expect("Var value should be string");
            vars_context.insert(key_str, value_str);
        }
    }
    return vars_context;
}

fn execute(app: App, app_name: &String, cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);

    let vars_context = get_vars_context(yaml);
    let args_context = get_args_context(&args, &subcmd_yaml);
    let mut context = HashMap::new();
    context.insert(String::from("vars"), vars_context);
    context.insert(String::from("args"), args_context);

    if app_name == "joat" && cmd_name == "init" {
        joat_scmds::execute_init(context);
        return;
    }

    if app_name == "joat" && cmd_name == "init" {
        joat_scmds::install(context);
        return;
    }

    if cmd_name == "auto_complete" {
        joat_scmds::execute_auto_complete(app, app_name, context);
        return;
    }

    let script = &subcmd_yaml["script"];
    if !script.is_badvalue() {
        script_scmd::execute_script(context, &subcmd_yaml, &yaml);
    } else {
        request_scmd::execute_request(&app_name, &cmd_name, &yaml, &subcmd_yaml, context);
    }
}

fn format_cmd_name(cmd_name: &String) -> String {
    let re = Regex::new("[^/]*$").unwrap();
    String::from(
        re.find(cmd_name)
        .expect("Failed to parse main cmd name")
        .as_str()
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = format_cmd_name(&args[0]);
    let config_yaml = yaml::get_yaml_config(&app_name);

    let version;
    if app_name == env!("CARGO_PKG_NAME") {
        version = String::from(
            config_yaml["version"]
            .as_str()
            .expect("Version not defined"));
    } else {
        let djoat_version = env!("CARGO_PKG_VERSION");;
        let app_version = config_yaml["version"].as_str().expect("Version not defined");
        version = format!("{} (joat {})", app_version, djoat_version);
    }
    let mut app = App::from_yaml(&config_yaml)
        .version(&*version);

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => {
                    execute(app, &app_name, name, sub_cmd, &config_yaml)
                },
                _ => {
                    // Could not find command, just print help
                    app.print_help().unwrap();
                    panic!("Could not find command");
                }
            }
        }
    }
}