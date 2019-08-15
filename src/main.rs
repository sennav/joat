extern crate clap;
extern crate dirs;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;

use clap::{App, ArgMatches};
use regex::Regex;
use serde_json::value::Value;
use std::collections::HashMap;
use std::env;
use yaml_rust::Yaml;

mod http;
mod joat_scmds;
mod oauth;
mod request_scmd;
mod script_scmd;
mod template;
mod yaml;

type InnerContext = HashMap<String, Value>;
type Context = HashMap<String, InnerContext>;

fn get_args_context(args: &ArgMatches, subcmd_yaml: &Yaml) -> InnerContext {
    let mut args_context = HashMap::new();
    for arg in subcmd_yaml["args"].clone().into_iter() {
        for a in arg.into_hash().unwrap().keys() {
            let key = a.clone().into_string().unwrap();
            if args.is_present(&key) {
                if let Some(m_values) = args.values_of(&key) {
                    let str_vec: Vec<&str> = m_values.collect();
                    let values: Vec<Value> = str_vec
                        .iter()
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    if values.len() > 1 {
                        args_context.insert(key, Value::Array(values));
                        continue;
                    }
                }
                if let Some(value) = args.value_of(&key) {
                    args_context.insert(key, Value::from(value));
                } else {
                    // Argument present, takes no value
                    args_context.insert(key, Value::from(true));
                }
            }
        }
    }
    return args_context;
}

fn get_vars_context(yaml: &Yaml) -> InnerContext {
    let mut vars_context = HashMap::new();
    let vars_yaml = &yaml["vars"];
    if !vars_yaml.is_badvalue() {
        let vars_iter = vars_yaml
            .clone()
            .into_hash()
            .expect("Could not convert vars into hash");
        for (key, value) in vars_iter {
            let key_str = key.clone().into_string().expect("Var key should be string");
            let value_str = value
                .clone()
                .into_string()
                .expect("Var value should be string");
            vars_context.insert(key_str, Value::from(value_str));
        }
    }
    return vars_context;
}

fn get_env_context() -> InnerContext {
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, Value::from(value));
    }
    return env_vars;
}

fn get_scmd_context(scmd_yaml: &Yaml) -> InnerContext {
    let mut scmd_vars = HashMap::new();
    match scmd_yaml["scmd_config_base_path"].clone().into_string() {
        Some(s) => {
            scmd_vars.insert(String::from("scmd_config_base_path"), Value::from(s));
        }
        None => (),
    }
    return scmd_vars;
}

fn execute(app: App, app_name: &String, cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);

    let vars_context = get_vars_context(yaml);
    let args_context = get_args_context(&args, &subcmd_yaml);
    let env_context = get_env_context();
    let scmd_context = get_scmd_context(&subcmd_yaml);
    let mut context: Context = HashMap::new();
    context.insert(String::from("vars"), vars_context);
    context.insert(String::from("args"), args_context);
    context.insert(String::from("env"), env_context);
    context.insert(String::from("scmd"), scmd_context);

    if app_name == "joat" && cmd_name == "init" {
        joat_scmds::execute_init(context);
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
            .as_str(),
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = format_cmd_name(&args[0]);
    let config_yaml = yaml::get_yaml_config(&app_name);

    let mut app = App::from_yaml(&config_yaml);

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => execute(app, &app_name, name, sub_cmd, &config_yaml),
                _ => {
                    // Could not find command, just print help
                    app.print_help().unwrap();
                    println!("");
                    ::std::process::exit(1);
                }
            }
        }
    }
}
